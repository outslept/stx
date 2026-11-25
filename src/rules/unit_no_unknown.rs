use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, at_rule_prelude_span, is_unicode_range_descriptor, media_feature_groups,
        options_matches, parse_option_matchers,
    },
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, Declaration, InterpolableIdent},
};
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Reports unknown units in declaration values and media features.
    ///
    /// Set to `true`. `ignoreUnits` and `ignoreFunctions` accept names or `/regex/`
    /// patterns; function exceptions apply to declarations. `x` is accepted in
    /// resolution contexts such as `image-set()` and `@media (resolution: 2x)`.
    /// Does not fix violations.
    ///
    /// Example: `@media (min-width: 13pix) {}` is reported for `pix`.
    "unit-no-unknown", correctness, recommended = true;
    pub struct UnitNoUnknown {
        ignore_units: Vec<OptionMatcher>,
        ignore_functions: Vec<OptionMatcher>,
    }
}

impl UnitNoUnknown {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_units: parse_option_matchers(secondary, "ignoreUnits"),
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
        }
    }
}

fn in_function(source: &str, offset: usize, name: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    let before = &lower[..offset.min(lower.len())];
    let Some(open) = before.rfind(&format!("{name}(")) else {
        return false;
    };
    let mut depth = 1usize;
    for byte in lower.as_bytes()[open + name.len() + 1..offset.min(lower.len())].iter() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth > 0
}

const KNOWN_UNITS: &[&str] = &[
    "%", "cap", "ch", "cm", "cqb", "cqh", "cqi", "cqmax", "cqmin", "cqw", "deg", "dpcm", "dpi",
    "dppx", "dvb", "dvh", "dvi", "dvmax", "dvmin", "dvw", "em", "ex", "fr", "grad", "hz", "ic",
    "in", "khz", "lh", "lvb", "lvh", "lvi", "lvmax", "lvmin", "lvw", "mm", "ms", "pc", "pt", "px",
    "q", "rad", "rcap", "rch", "rem", "rex", "ric", "rlh", "s", "svb", "svh", "svi", "svmax",
    "svmin", "svw", "turn", "vb", "vh", "vi", "vmax", "vmin", "vw",
];

impl Rule for UnitNoUnknown {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if is_unicode_range_descriptor(prop, &ctx.ancestors) {
            return;
        }
        let prop_end = ident.span.end as usize;
        let end = decl.span.end as usize;
        let src = ctx.source;
        if prop_end >= end || end > src.len() {
            return;
        }
        let value = &src[prop_end..end];
        if value.contains('$')
            || value.contains('@')
            || value.contains("#{")
            || value.contains("@{")
        {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = prop_end;

        while i < end && bytes[i] != b':' {
            i += 1;
        }
        if i < end && bytes[i] == b':' {
            i += 1;
        }

        while i < end {
            if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                i += 2;
                while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(end);
                continue;
            }

            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                i += 1;
                while i < end {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            if i + 4 <= end && bytes[i..i + 4].eq_ignore_ascii_case(b"url(") {
                i += 4;
                let mut depth = 1usize;
                while i < end && depth > 0 {
                    if bytes[i] == b'(' {
                        depth += 1;
                    } else if bytes[i] == b')' {
                        depth -= 1;
                    }
                    i += 1;
                }
                continue;
            }

            if bytes[i] == b'#' {
                i += 1;
                while i < end && bytes[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                continue;
            }

            if bytes[i].is_ascii_alphabetic()
                || bytes[i] == b'_'
                || (bytes[i] == b'-' && !bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
            {
                let start = i;
                while i < end
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
                {
                    i += 1;
                }
                if i < end && bytes[i] == b'(' {
                    let name = &src[start..i];
                    if name.eq_ignore_ascii_case("url")
                        || options_matches(&self.ignore_functions, name)
                    {
                        i += 1;
                        let mut depth = 1usize;
                        let mut quote = 0u8;
                        while i < end && depth > 0 {
                            if quote != 0 {
                                if bytes[i] == b'\\' {
                                    i += 2;
                                    continue;
                                }
                                if bytes[i] == quote {
                                    quote = 0;
                                }
                            } else if bytes[i] == b'\'' || bytes[i] == b'"' {
                                quote = bytes[i];
                            } else if bytes[i] == b'(' {
                                depth += 1;
                            } else if bytes[i] == b')' {
                                depth -= 1;
                            }
                            i += 1;
                        }
                        continue;
                    }
                }
                continue;
            }

            if bytes[i].is_ascii_digit() {
                while i < end && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i < end && bytes[i] == b'.' && i + 1 < end && bytes[i + 1].is_ascii_digit() {
                    i += 1;
                    while i < end && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if i < end
                    && (bytes[i] == b'e' || bytes[i] == b'E')
                    && i + 1 < end
                    && (bytes[i + 1].is_ascii_digit()
                        || ((bytes[i + 1] == b'+' || bytes[i + 1] == b'-')
                            && i + 2 < end
                            && bytes[i + 2].is_ascii_digit()))
                {
                    i += 2;
                    while i < end && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }

                if i < end && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'%') {
                    let unit_start = i;
                    if bytes[i] == b'%' {
                        i += 1;
                    } else {
                        while i < end
                            && (bytes[i].is_ascii_alphanumeric()
                                || bytes[i] == b'-'
                                || bytes[i] == b'_')
                        {
                            i += 1;
                        }
                    }
                    let unit_raw = &src[unit_start..i];
                    let unit_lower = unit_raw.to_ascii_lowercase();
                    if options_matches(&self.ignore_units, unit_raw) {
                        continue;
                    }
                    if !KNOWN_UNITS.contains(&unit_lower.as_str())
                        && !(unit_lower == "x"
                            && (prop.eq_ignore_ascii_case("image-resolution")
                                || in_function(src, unit_start, "image-set")))
                    {
                        ctx.report(
                            self.name(),
                            Span {
                                start: unit_start as u32,
                                end: i as u32,
                            },
                            format!("Unknown unit \"{unit_raw}\""),
                        );
                    }
                }
                continue;
            }

            i += 1;
        }
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("media") {
            return;
        }
        let Some(span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let params = ctx.slice(span);
        for group in media_feature_groups(params) {
            let name = &params[group.name_start..group.name_end];
            for (_, end, unit_start) in group.dimensions {
                let unit = &params[unit_start..end];
                let lower = unit.to_ascii_lowercase();
                if options_matches(&self.ignore_units, unit)
                    || KNOWN_UNITS.contains(&lower.as_str())
                    || (lower == "x"
                        && (name.to_ascii_lowercase().contains("resolution")
                            || name.to_ascii_lowercase().contains("pixel-ratio")))
                {
                    continue;
                }
                ctx.report(
                    self.name(),
                    Span {
                        start: span.start + unit_start as u32,
                        end: span.start + end as u32,
                    },
                    format!("Unknown unit \"{unit}\""),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn checks_media_units_and_skips_comments_and_unicode_ranges() {
        let fail = [
            reject(
                "@media (min-width: 13pix) {}",
                None,
                "Unknown unit \"pix\"",
                (1, 22, 1, 25),
            ),
            reject(
                "@font-face { color: U+0100-024F; }",
                None,
                "Unknown unit \"F\"",
                (1, 31, 1, 32),
            ),
            reject(
                "a { unicode-range: U+0100-024F; }",
                None,
                "Unknown unit \"F\"",
                (1, 30, 1, 31),
            ),
            reject(
                "a { width: -1pix; }",
                None,
                "Unknown unit \"pix\"",
                (1, 14, 1, 17),
            ),
        ];
        Tester::new(
            "unit-no-unknown",
            json!(true),
            [
                "a { width: /* 100pix */ 1em; }",
                "@font-face { unicode-range: U+0100-024F; }",
                "@media (resolution: 2x) {}",
            ],
            &fail,
        )
        .test();
    }
}

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        NAMED_COLORS_KEYWORDS, SYSTEM_COLORS_KEYWORDS, UNITS, ValueToken, ValueTokenKind,
        decl_value_span, ident_name, in_set_ignore_case, is_custom_property, is_valid_hex,
        tokenize_value,
    },
};
use oxc_css_parser::{Span, ast::Declaration};

pub struct DeclarationPropertyValueNoUnknown;

fn allowed_keywords(prop: &str) -> Option<&'static [&'static str]> {
    match prop {
        "text-wrap" => Some(&["wrap", "nowrap", "balance", "stable", "pretty"]),
        "text-box-trim" => Some(&["trim-start", "trim-end", "trim-both", "none"]),
        "text-spacing-trim" => Some(&[
            "space-all",
            "normal",
            "space-first",
            "trim-start",
            "trim-both",
        ]),
        "field-sizing" => Some(&["content", "fixed"]),
        _ => None,
    }
}

fn valid_dashed_names(prop: &str, value: &str) -> bool {
    let names = value
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty());
    let mut found = false;
    for name in names {
        found = true;
        if name.eq_ignore_ascii_case("none")
            || (prop == "position-anchor" && name.eq_ignore_ascii_case("auto"))
        {
            if value.trim().eq_ignore_ascii_case(name) {
                return true;
            }
        } else if !name.starts_with("--") || name.len() == 2 {
            return false;
        }
    }
    found
}

fn is_global_keyword(value: &str) -> bool {
    ["initial", "inherit", "unset", "revert", "revert-layer"]
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case(value))
}

fn is_length(token: &ValueToken<'_>, percentage: bool) -> bool {
    match token.kind {
        ValueTokenKind::Number => token.raw.parse::<f64>().is_ok_and(|number| number == 0.0),
        ValueTokenKind::Percentage => percentage,
        ValueTokenKind::Dimension { num_len } => {
            let unit = &token.raw[num_len..];
            token.raw[..num_len].parse::<f64>().is_ok() && in_set_ignore_case(UNITS, unit)
        }
        _ => false,
    }
}

fn is_sizing_keyword(value: &str) -> bool {
    [
        "none",
        "min-content",
        "max-content",
        "fit-content",
        "stretch",
        "min-intrinsic",
        "available",
    ]
    .iter()
    .any(|keyword| keyword.eq_ignore_ascii_case(value))
}

fn known_length_property(prop: &str, value: &str, tokens: &[ValueToken<'_>]) -> Option<bool> {
    let (max_values, auto) = match prop {
        "width"
        | "height"
        | "min-width"
        | "max-width"
        | "min-height"
        | "max-height"
        | "top"
        | "right"
        | "bottom"
        | "left"
        | "inset-block-start"
        | "inset-block-end"
        | "inset-inline-start"
        | "inset-inline-end"
        | "margin-top"
        | "margin-right"
        | "margin-bottom"
        | "margin-left"
        | "margin-block-start"
        | "margin-block-end"
        | "margin-inline-start"
        | "margin-inline-end" => (1, true),
        "padding-top"
        | "padding-right"
        | "padding-bottom"
        | "padding-left"
        | "padding-block-start"
        | "padding-block-end"
        | "padding-inline-start"
        | "padding-inline-end" => (1, false),
        "inset-block" | "inset-inline" | "margin-block" | "margin-inline" => (2, true),
        "padding-block" | "padding-inline" => (2, false),
        "inset" | "margin" => (4, true),
        "padding" => (4, false),
        "font-size" => {
            return Some(
                tokens.len() == 1
                    && (is_length(&tokens[0], true)
                        || [
                            "xx-small",
                            "x-small",
                            "small",
                            "medium",
                            "large",
                            "x-large",
                            "xx-large",
                            "xxx-large",
                            "larger",
                            "smaller",
                        ]
                        .iter()
                        .any(|keyword| keyword.eq_ignore_ascii_case(tokens[0].raw))),
            );
        }
        _ => return None,
    };

    if value.contains('(') {
        return Some(true);
    }
    if tokens.is_empty() {
        return Some(false);
    }
    if tokens.len() == 1 && is_global_keyword(value.trim()) {
        return Some(true);
    }
    if tokens.len() > max_values {
        return Some(false);
    }
    Some(tokens.iter().all(|token| {
        (auto && token.kind == ValueTokenKind::Ident && token.raw.eq_ignore_ascii_case("auto"))
            || (token.kind == ValueTokenKind::Ident
                && is_sizing_keyword(token.raw)
                && (!token.raw.eq_ignore_ascii_case("none")
                    || prop == "max-width"
                    || prop == "max-height"))
            || is_length(token, true)
    }))
}

fn color_count(tokens: &[ValueToken<'_>]) -> Option<usize> {
    let mut count = 0;
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        if token.raw == "#" {
            let next = tokens.get(i + 1)?;
            if token.span.end != next.span.start
                || next.kind != ValueTokenKind::Ident
                || !is_valid_hex(&format!("#{}", next.raw))
            {
                return None;
            }
            i += 2;
        } else if token.kind == ValueTokenKind::Ident
            && (token.raw.eq_ignore_ascii_case("transparent")
                || token.raw.eq_ignore_ascii_case("currentcolor")
                || in_set_ignore_case(NAMED_COLORS_KEYWORDS, token.raw)
                || in_set_ignore_case(SYSTEM_COLORS_KEYWORDS, token.raw))
        {
            i += 1;
        } else {
            return None;
        }
        count += 1;
    }
    Some(count)
}

fn known_value(prop: &str, value: &str, tokens: &[ValueToken<'_>]) -> Option<bool> {
    if value.contains('(') {
        return Some(true);
    }
    if let Some(allowed) = allowed_keywords(prop) {
        return Some(
            tokens.len() == 1
                && tokens[0].kind == ValueTokenKind::Ident
                && (allowed
                    .iter()
                    .any(|keyword| keyword.eq_ignore_ascii_case(tokens[0].raw))
                    || is_global_keyword(tokens[0].raw)),
        );
    }
    if [
        "view-timeline-name",
        "view-transition-name",
        "anchor-name",
        "position-anchor",
    ]
    .contains(&prop)
    {
        return Some(valid_dashed_names(prop, value.trim()) || is_global_keyword(value.trim()));
    }
    if is_global_keyword(value.trim()) {
        return Some(true);
    }
    if crate::utils::COLOR_PROPERTIES.contains(&prop) {
        let Some(count) = color_count(tokens) else {
            return Some(false);
        };
        let max = if prop == "border-color" {
            4
        } else if prop == "scrollbar-color" {
            2
        } else {
            1
        };
        let min = if prop == "scrollbar-color" { 2 } else { 1 };
        return Some(count >= min && count <= max);
    }
    if let Some(valid) = known_length_property(prop, value, tokens) {
        return Some(valid);
    }
    match prop {
        "display" => {
            if tokens.iter().any(|token| token.raw.starts_with('-')) {
                return None;
            }
            Some(
                !tokens.is_empty()
                    && tokens.len() <= 2
                    && tokens.iter().all(|token| {
                        token.kind == ValueTokenKind::Ident
                            && [
                                "none",
                                "contents",
                                "block",
                                "inline",
                                "run-in",
                                "flow",
                                "flow-root",
                                "table",
                                "flex",
                                "grid",
                                "ruby",
                                "inline-block",
                                "inline-table",
                                "inline-flex",
                                "inline-grid",
                                "list-item",
                                "legacy",
                                "-moz-inline-stack",
                            ]
                            .iter()
                            .any(|keyword| keyword.eq_ignore_ascii_case(token.raw))
                    }),
            )
        }
        "position" => Some(
            tokens.len() == 1
                && ["static", "relative", "absolute", "fixed", "sticky"]
                    .iter()
                    .any(|keyword| keyword.eq_ignore_ascii_case(tokens[0].raw)),
        ),
        "visibility" => Some(
            tokens.len() == 1
                && ["visible", "hidden", "collapse"]
                    .iter()
                    .any(|keyword| keyword.eq_ignore_ascii_case(tokens[0].raw)),
        ),
        "box-sizing" => Some(
            tokens.len() == 1
                && ["content-box", "border-box"]
                    .iter()
                    .any(|keyword| keyword.eq_ignore_ascii_case(tokens[0].raw)),
        ),
        "overflow" | "overflow-x" | "overflow-y" | "overflow-block" | "overflow-inline" => Some(
            !tokens.is_empty()
                && tokens.len() <= 2
                && tokens.iter().all(|token| {
                    token.kind == ValueTokenKind::Ident
                        && ["visible", "hidden", "clip", "scroll", "auto", "overlay"]
                            .iter()
                            .any(|keyword| keyword.eq_ignore_ascii_case(token.raw))
                }),
        ),
        "opacity" => Some(
            tokens.len() == 1
                && matches!(
                    tokens[0].kind,
                    ValueTokenKind::Number | ValueTokenKind::Percentage
                ),
        ),
        _ => None,
    }
}

impl Rule for DeclarationPropertyValueNoUnknown {
    fn name(&self) -> &'static str {
        "declaration-property-value-no-unknown"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !ctx.is_standard_declaration(decl) || is_custom_property(prop) {
            return;
        }
        let Some(span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let value = ctx.slice(span);
        let tokens = tokenize_value(ctx.source, span);
        let Some(valid) = known_value(&prop.to_ascii_lowercase(), value, &tokens) else {
            return;
        };
        if valid {
            return;
        }
        // css-tree points at the first mismatching node, skipping leading comments.
        let lower = prop.to_ascii_lowercase();
        let bad = tokens.iter().position(|token| {
            known_length_property(&lower, token.raw, std::slice::from_ref(token)) == Some(false)
        });
        let report_span = match (bad, tokens.first()) {
            (Some(i), _) => tokens[i].span,
            (None, Some(first)) => Span {
                start: first.span.start,
                end: span.end,
            },
            (None, None) => {
                let rest = &ctx.source[span.start as usize..];
                let start = span.start + (rest.len() - rest.trim_start().len()) as u32;
                Span { start, end: start }
            }
        };
        ctx.report(
            self.name(),
            report_span,
            format!(
                "Unknown value \"{}\" for property \"{prop}\"",
                ctx.slice(report_span).trim()
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a { color: red; }",
            "a { text-wrap: balance; }",
            "a { view-transition-name: --x; }",
            "a { view-transition-name: none; }",
            "a { width: calc(100% - 10px); }",
            "a { margin: auto 10em; }",
            "a { color: var(--color); }",
            "a { position-anchor: var(--anchor); }",
            "a { overflow: overlay; }",
            "a { display: inline flex; }",
            "a { display: -moz-inline-stack; }",
            "a { padding-top: 0; }",
            "a { padding: 0 2px; }",
            "a { width: min-content; }",
            "a { border-color: red #fff blue green; }",
            "a { scrollbar-color: red blue; }",
        ];
        let fail = vec![
            reject_message(
                "a { text-wrap: foo; }",
                None,
                "Unknown value \"foo\" for property \"text-wrap\"",
            ),
            reject_message(
                "a { view-timeline-name: foo; }",
                None,
                "Unknown value \"foo\" for property \"view-timeline-name\"",
            ),
            reject_message(
                "a { text-spacing-trim: bar; }",
                None,
                "Unknown value \"bar\" for property \"text-spacing-trim\"",
            ),
            reject_message(
                "a { top: red; }",
                None,
                "Unknown value \"red\" for property \"top\"",
            ),
            reject_message(
                "a { color: greem; }",
                None,
                "Unknown value \"greem\" for property \"color\"",
            ),
            reject_message(
                "a { display: fixed; }",
                None,
                "Unknown value \"fixed\" for property \"display\"",
            ),
            reject_message(
                "a { padding-top: auto; }",
                None,
                "Unknown value \"auto\" for property \"padding-top\"",
            ),
            reject_message(
                "a { scrollbar-color: red; }",
                None,
                "Unknown value \"red\" for property \"scrollbar-color\"",
            ),
        ];
        Tester::new(
            "declaration-property-value-no-unknown",
            json!(true),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn reports_the_mismatching_part_of_the_value() {
        let fail = [
            reject(
                "a {\n\ttop: /* a comment */\n\t\tred;\n}",
                None,
                "Unknown value \"red\" for property \"top\"",
                (3, 3, 3, 6),
            ),
            reject(
                "a { color: ; }",
                None,
                "Unknown value \"\" for property \"color\"",
                (1, 12, 1, 13),
            ),
            reject(
                "a { padding: 10px --bar; }",
                None,
                "Unknown value \"--bar\" for property \"padding\"",
                (1, 19, 1, 24),
            ),
        ];
        Tester::new(
            "declaration-property-value-no-unknown",
            json!(true),
            [],
            fail,
        )
        .test();
    }
}

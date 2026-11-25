use std::collections::HashSet;

use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;

#[derive(Debug)]
enum IgnoreMatcher {
    Exact(String),
    Regex(Regex),
}

impl IgnoreMatcher {
    fn from_str(s: &str) -> Option<Self> {
        if s.starts_with('/')
            && s.len() >= 2
            && let Some(last_slash) = s.rfind('/')
            && last_slash > 0
        {
            let pattern = &s[1..last_slash];
            let flags = &s[last_slash + 1..];
            let mut builder = RegexBuilder::new(pattern);
            if flags.contains('i') {
                builder.case_insensitive(true);
            }
            return builder.build().ok().map(IgnoreMatcher::Regex);
        }
        Some(IgnoreMatcher::Exact(s.to_string()))
    }

    fn matches(&self, prop: &str) -> bool {
        match self {
            IgnoreMatcher::Exact(s) => s == prop,
            IgnoreMatcher::Regex(re) => re.is_match(prop),
        }
    }
}

crate::define_stx_lint! {
    /// Disallows repeated custom property names within the same declaration block.
    ///
    /// Set the primary option to `true`. Set `ignoreProperties` to a name, a
    /// `/regex/` (optionally with `i`), or an array of either. Literal names are
    /// case-sensitive.
    ///
    /// ```css
    /// a { --accent: red; --accent: blue; } /* reported */
    /// ```
    "declaration-block-no-duplicate-custom-properties", suspicious, recommended = true;
    #[derive(Debug, Default)]
    pub struct DeclarationBlockNoDuplicateCustomProperties {
        ignore_properties: Vec<IgnoreMatcher>,
    }
}

impl DeclarationBlockNoDuplicateCustomProperties {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Option<Self> {
        if primary.as_bool() == Some(false) {
            return None;
        }

        let mut ignore_properties = Vec::new();
        if let Some(sec) = secondary
            && let Some(ig) = sec.get("ignoreProperties")
        {
            match ig {
                Value::String(s) => {
                    if let Some(m) = IgnoreMatcher::from_str(s) {
                        ignore_properties.push(m);
                    }
                }
                Value::Array(arr) => {
                    for item in arr {
                        if let Some(s) = item.as_str()
                            && let Some(m) = IgnoreMatcher::from_str(s)
                        {
                            ignore_properties.push(m);
                        }
                    }
                }
                _ => {}
            }
        }

        Some(Self { ignore_properties })
    }

    fn is_ignored(&self, prop: &str) -> bool {
        self.ignore_properties.iter().any(|m| m.matches(prop))
    }
}

pub fn rejected_message(property: &str) -> String {
    format!("Duplicate custom property \"{property}\"")
}

fn is_standard_syntax_property(prop: &str) -> bool {
    !prop.starts_with('$')
        && !prop.starts_with('@')
        && !prop.contains("#{")
        && !prop.contains("@{")
        && !prop.contains("$(")
}

fn skip_braces(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                i += 1;
                break;
            }
        }
        i += 1;
    }
    i.min(len)
}

impl Rule for DeclarationBlockNoDuplicateCustomProperties {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let source = ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();

        let mut stack: Vec<HashSet<String>> = Vec::new();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            if b == b'{' {
                stack.push(HashSet::new());
                i += 1;
                continue;
            }

            if b == b'}' {
                stack.pop();
                i += 1;
                continue;
            }

            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                continue;
            }

            let stmt_start = i;
            let mut paren_depth = 0usize;
            let mut first_colon: Option<usize> = None;

            while i < len {
                let c = bytes[i];
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == b'"' || c == b'\'' {
                    let quote = c;
                    i += 1;
                    while i < len {
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
                if c == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                    i += 2;
                    while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                        i += 1;
                    }
                    i = (i + 2).min(len);
                    continue;
                }
                if c == b'{' && i > stmt_start && (bytes[i - 1] == b'#' || bytes[i - 1] == b'@') {
                    i = skip_braces(bytes, i);
                    continue;
                }
                if c == b'(' {
                    paren_depth += 1;
                } else if c == b')' {
                    paren_depth = paren_depth.saturating_sub(1);
                } else if paren_depth == 0 {
                    if c == b':' && first_colon.is_none() {
                        first_colon = Some(i);
                    } else if c == b'{' || c == b';' || c == b'}' {
                        break;
                    }
                }
                i += 1;
            }

            if i < len && bytes[i] == b'{' {
                stack.push(HashSet::new());
                i += 1;
            } else {
                let stmt_slice = &source[stmt_start..i];
                if !stmt_slice.starts_with('@')
                    && let Some(colon_pos) = first_colon
                {
                    let prop = source[stmt_start..colon_pos].trim();
                    if prop.starts_with("--")
                        && prop.len() > 2
                        && is_standard_syntax_property(prop)
                        && !self.is_ignored(prop)
                        && let Some(current_set) = stack.last_mut()
                        && !current_set.insert(prop.to_string())
                    {
                        let start = stmt_start as u32;
                        let end = start + prop.len() as u32;
                        ctx.report(self.name(), Span { start, end }, rejected_message(prop));
                    }
                }
                if i < len && bytes[i] == b';' {
                    i += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_message};

    const NAME: &str = "declaration-block-no-duplicate-custom-properties";

    fn rejected(property: &str) -> String {
        format!("Duplicate custom property \"{property}\"")
    }

    #[test]
    fn declaration_block_no_duplicate_custom_properties_default() {
        let pass = [
            "a { --custom-property: 1 }",
            "a { --custom-property: 1; --cUstOm-prOpErtY: 1 }",
            "a { --custom-property: 1; color: pink; --cUstOm-prOpErtY: 1 }",
            "a { color: var(--custom-property, --custom-property) }",
            "a { --custom-property: pink; @media { --custom-property: orange; } }",
            "a { --custom-property: pink; @media { --custom-property: orange; &::before { --custom-property: black; } } }",
            "a { --custom-property: pink; { &:hover { --custom-property: orange; --cUstOm-prOpErtY: black; } } }",
            "a { --cUstOm-prOpErtY: pink; { &:hover { --custom-property: orange; --cUstOm-prOpErtY: black; } } }",
        ];
        let fail = [
            reject(
                "a { --custom-property: 1; --custom-property: 2; }",
                None,
                rejected("--custom-property"),
                (1, 27, 1, 44),
            ),
            reject(
                "a { --custom-property: 1; color: pink; --custom-property: 1; }",
                None,
                rejected("--custom-property"),
                (1, 40, 1, 57),
            ),
            reject(
                "a { --custom-property: 1; --cUstOm-prOpErtY: 1; color: pink; --cUstOm-prOpErtY: 1; }",
                None,
                rejected("--cUstOm-prOpErtY"),
                (1, 62, 1, 79),
            ),
            reject(
                "a { --custom-property: pink; { &:hover { --custom-property: orange; --custom-property: black; } } }",
                None,
                rejected("--custom-property"),
                (1, 69, 1, 86),
            ),
            reject(
                "a { --custom-property: pink; @media { --custom-property: orange; --custom-property: black; } }",
                None,
                rejected("--custom-property"),
                (1, 66, 1, 83),
            ),
            reject(
                "@media { --custom-property: orange; .foo { --custom-property: black; --custom-property: white; } }",
                None,
                rejected("--custom-property"),
                (1, 70, 1, 87),
            ),
            reject(
                "a { --custom-property: pink; @media { --custom-property: orange; &::before { --custom-property: black; --custom-property: white; } } }",
                None,
                rejected("--custom-property"),
                (1, 104, 1, 121),
            ),
            reject(
                "a { --custom-property: pink; @media { --custom-property: orange; .foo { --custom-property: black; --custom-property: white; } } }",
                None,
                rejected("--custom-property"),
                (1, 99, 1, 116),
            ),
        ];
        Tester::new(NAME, json!([true]), pass, fail).test();
    }

    #[test]
    #[ignore = "postcss-html syntax is not supported"]
    fn declaration_block_no_duplicate_custom_properties_html() {
        let pass = ["<style>a { --custom-property: pink; }</style>"];
        Tester::new(NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-html")
            .test();
    }

    #[test]
    #[ignore = "naive-css-in-js syntax is not supported"]
    fn declaration_block_no_duplicate_custom_properties_css_in_js() {
        let pass = ["css` --custom-property: pink; `;"];
        Tester::new(NAME, json!([true]), pass, [])
            .with_custom_syntax("naive-css-in-js")
            .test();
    }

    #[test]
    fn declaration_block_no_duplicate_custom_properties_ignore_properties() {
        let pass = [
            "a { --custom-property: 1; --custom-property: 1; }",
            "a { --custom-ignored-property: 1; --custom-ignored-property: 1; }",
        ];
        let fail = [reject_message(
            "a { --another-custom-property: 1; --another-custom-property: 1; }",
            None,
            rejected("--another-custom-property"),
        )];
        Tester::new(
            NAME,
            json!([true, { "ignoreProperties": ["--custom-property", "/ignored/"] }]),
            pass,
            fail,
        )
        .test();
    }
}

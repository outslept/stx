use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;

crate::define_stx_lint! {
    /// Require custom property names to match a regular expression.
    ///
    /// Supply a bare regex string or `/pattern/` with optional `i` as the
    /// primary option. There are no secondary options. The match excludes
    /// the initial `--` and checks declarations, `@property` registrations,
    /// and names used as the first argument to `var()`. Nonstandard property
    /// syntax, such as preprocessor interpolation, is skipped.
    ///
    /// With `"^[a-z-]+$"`:
    /// ```css
    /// :root { --AccentColor: red; } /* reported */
    /// :root { --accent-color: red; } /* accepted */
    /// ```
    "custom-property-pattern", style;
    #[derive(Debug)]
    pub struct CustomPropertyPattern {
        regex: Regex,
        raw_pattern: String,
    }
}

impl CustomPropertyPattern {
    pub fn from_config(primary: &Value, _secondary: Option<&Map<String, Value>>) -> Option<Self> {
        let raw_pattern = primary.as_str()?.to_string();
        let regex = compile_pattern(&raw_pattern)?;
        Some(Self { regex, raw_pattern })
    }

    fn check_prop(&self, property: &str) -> bool {
        if !property.starts_with("--") {
            return true;
        }
        if !is_standard_syntax_property(property) {
            return true;
        }
        self.regex.is_match(&property[2..])
    }
}

fn compile_pattern(raw: &str) -> Option<Regex> {
    if raw.starts_with('/')
        && raw.len() >= 2
        && let Some(last_slash) = raw.rfind('/')
        && last_slash > 0
    {
        let pattern = &raw[1..last_slash];
        let flags = &raw[last_slash + 1..];
        let mut builder = RegexBuilder::new(pattern);
        if flags.contains('i') {
            builder.case_insensitive(true);
        }
        return builder.build().ok();
    }
    Regex::new(raw).ok()
}

pub fn expected_message(property: &str, pattern: &str) -> String {
    format!("Expected \"{property}\" to match pattern \"{pattern}\"")
}

fn is_standard_syntax_property(prop: &str) -> bool {
    !prop.starts_with('$')
        && !prop.starts_with('@')
        && !prop.contains("#{")
        && !prop.contains("@{")
        && !prop.contains("$(")
}

#[derive(Debug)]
enum ValNode<'a> {
    Word {
        value: &'a str,
        offset: usize,
    },
    Div,
    Function {
        name: &'a str,
        args: Vec<ValNode<'a>>,
    },
}

fn skip_parens(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == b'"' || b == b'\'' {
            let quote = b;
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
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
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

fn parse_value_nodes<'a>(s: &'a str, base_offset: usize) -> Vec<ValNode<'a>> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut nodes = Vec::new();

    while i < len {
        let b = bytes[i];

        if b.is_ascii_whitespace() {
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

        if b == b'"' || b == b'\'' {
            let quote = b;
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

        if b == b',' || b == b'/' || b == b':' {
            nodes.push(ValNode::Div);
            i += 1;
            continue;
        }

        if b == b'(' {
            let open_paren = i;
            let close_paren = skip_parens(bytes, open_paren);
            let inner_end = if close_paren > open_paren + 1 && bytes[close_paren - 1] == b')' {
                close_paren - 1
            } else {
                close_paren
            };
            let args =
                parse_value_nodes(&s[open_paren + 1..inner_end], base_offset + open_paren + 1);
            nodes.push(ValNode::Function { name: "", args });
            i = close_paren;
            continue;
        }

        if b == b')' {
            i += 1;
            continue;
        }

        let start = i;
        while i < len {
            let c = bytes[i];
            if c == b'\\' && i + 1 < len {
                i += 2;
                continue;
            }
            if c == b'{' && i > start && (bytes[i - 1] == b'#' || bytes[i - 1] == b'@') {
                i = skip_braces(bytes, i);
                continue;
            }
            if c.is_ascii_whitespace()
                || c == b','
                || c == b'/'
                || c == b':'
                || c == b'('
                || c == b')'
                || c == b'"'
                || c == b'\''
            {
                break;
            }
            i += 1;
        }

        if i < len && bytes[i] == b'(' {
            let func_name = &s[start..i];
            let open_paren = i;
            let close_paren = skip_parens(bytes, open_paren);
            let inner_end = if close_paren > open_paren + 1 && bytes[close_paren - 1] == b')' {
                close_paren - 1
            } else {
                close_paren
            };
            let args =
                parse_value_nodes(&s[open_paren + 1..inner_end], base_offset + open_paren + 1);
            nodes.push(ValNode::Function {
                name: func_name,
                args,
            });
            i = close_paren;
        } else if i > start {
            nodes.push(ValNode::Word {
                value: &s[start..i],
                offset: base_offset + start,
            });
        }
    }

    nodes
}

impl CustomPropertyPattern {
    fn walk_value_node<'a>(
        &self,
        node: &ValNode<'a>,
        value_start: usize,
        ctx: &mut LintContext<'a>,
    ) {
        if let ValNode::Function { name, args } = node {
            if name.eq_ignore_ascii_case("var")
                && let Some(ValNode::Word { value, offset }) = args.first()
                && !self.check_prop(value)
            {
                let start = (value_start + *offset) as u32;
                let end = start + value.len() as u32;
                ctx.report(
                    self.name(),
                    Span { start, end },
                    expected_message(value, &self.raw_pattern),
                );
            }
            for arg in args {
                self.walk_value_node(arg, value_start, ctx);
            }
        }
    }
}

impl Rule for CustomPropertyPattern {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let source = ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();

        let mut block_depth = 0usize;
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            if b == b'}' {
                block_depth = block_depth.saturating_sub(1);
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

            let stmt_slice = &source[stmt_start..i];

            if i < len && bytes[i] == b'{' {
                // Check `@property` at-rules
                if let Some(rest) = stmt_slice.strip_prefix('@') {
                    let name_len = rest
                        .bytes()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_')
                        .count();
                    let at_name = &rest[..name_len];
                    if at_name.eq_ignore_ascii_case("property") {
                        let raw_params = &rest[name_len..];
                        let trimmed_start = raw_params.len() - raw_params.trim_start().len();
                        let params_trimmed = raw_params.trim();
                        let params_abs_start = stmt_start + 1 + name_len + trimmed_start;

                        for node in parse_value_nodes(params_trimmed, 0) {
                            if let ValNode::Word { value, offset } = node
                                && !self.check_prop(value)
                            {
                                let start = (params_abs_start + offset) as u32;
                                let end = start + value.len() as u32;
                                ctx.report(
                                    self.name(),
                                    Span { start, end },
                                    expected_message(value, &self.raw_pattern),
                                );
                            }
                        }
                    }
                }
                block_depth += 1;
                i += 1;
            } else {
                if block_depth > 0
                    && !stmt_slice.starts_with('@')
                    && let Some(colon_pos) = first_colon
                {
                    let prop_raw = &source[stmt_start..colon_pos];
                    let prop = prop_raw.trim();

                    let raw_val = &source[colon_pos + 1..i];
                    let val_trimmed_start = raw_val.len() - raw_val.trim_start().len();
                    let val_trimmed = raw_val.trim();
                    let val_start = colon_pos + 1 + val_trimmed_start;

                    if val_trimmed.to_ascii_lowercase().contains("var(") {
                        let nodes = parse_value_nodes(val_trimmed, 0);
                        for node in &nodes {
                            self.walk_value_node(node, val_start, ctx);
                        }
                    }

                    if !self.check_prop(prop) {
                        let start = stmt_start as u32;
                        let end = start + prop.len() as u32;
                        ctx.report(
                            self.name(),
                            Span { start, end },
                            expected_message(prop, &self.raw_pattern),
                        );
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

    use crate::tester::{Tester, reject, reject_warnings};

    const NAME: &str = "custom-property-pattern";

    fn expected(property: &str, pattern: &str) -> String {
        format!("Expected \"{property}\" to match pattern \"{pattern}\"")
    }

    #[test]
    fn custom_property_pattern_regex_foo() {
        let pattern = "/foo-.+/";
        let pass = [
            ":root { --foo-bar: 0; }",
            ":root { --boo-foo-bar: 0; }",
            ":root { --foo-color: #f00; } a { color: var(--foo-color); }",
            ":root { --foo-sub-color: #f00; } a { color: var(--foo-color, var(--foo-sub-color)); }",
            "a { --foo-color: var(); }",
        ];
        let fail = [
            reject(
                ":root { --boo-bar: 0; }",
                None,
                expected("--boo-bar", pattern),
                (1, 9, 1, 18),
            ),
            reject(
                ":root { --foo-: 0; }",
                None,
                expected("--foo-", pattern),
                (1, 9, 1, 15),
            ),
            reject(
                ":root { --foo-color: #f00; } a { color: var(--boo-color); }",
                None,
                expected("--boo-color", pattern),
                (1, 45, 1, 56),
            ),
            reject(
                ":root { --foo-sub-color: #f00; } a { color: var(--foo-color, var(--boo-sub-color)); }",
                None,
                expected("--boo-sub-color", pattern),
                (1, 66, 1, 81),
            ),
            reject(
                ":root { --foo-color: #f00; } a { color: VAR(--boo-color)); }",
                None,
                expected("--boo-color", pattern),
                (1, 45, 1, 56),
            ),
            reject_warnings(
                ":root { --foo-color: #f00; } a { color: var(--boo-color), var(--boo-sub-color)); }",
                None,
                vec![
                    (expected("--boo-color", pattern), (1, 45, 1, 56)),
                    (expected("--boo-sub-color", pattern), (1, 63, 1, 78)),
                ],
            ),
        ];
        Tester::new(NAME, json!([pattern]), pass, fail).test();
    }

    #[test]
    fn custom_property_pattern_string_foo() {
        let pattern = "foo-.+";
        let pass = [":root { --foo-bar: 0; }", ":root { --boo-foo-bar: 0; }"];
        let fail = [
            reject(
                ":root { --boo-bar: 0; }",
                None,
                expected("--boo-bar", pattern),
                (1, 9, 1, 18),
            ),
            reject(
                ":root { --foo-: 0; }",
                None,
                expected("--foo-", pattern),
                (1, 9, 1, 15),
            ),
        ];
        Tester::new(NAME, json!([pattern]), pass, fail).test();
    }

    #[test]
    fn custom_property_pattern_case_sensitive() {
        let pattern = "/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/";
        let pass = [":root { --Foo-bar: 0; }", ":root { --Foo-barBaz: 0; }"];
        let fail = [
            reject(
                ":root { --boo-Foo-bar: 0; }",
                None,
                expected("--boo-Foo-bar", pattern),
                (1, 9, 1, 22),
            ),
            reject(
                ":root { --foo-bar: 0; }",
                None,
                expected("--foo-bar", pattern),
                (1, 9, 1, 18),
            ),
            reject(
                ":root { --Foo-Bar: 0; }",
                None,
                expected("--Foo-Bar", pattern),
                (1, 9, 1, 18),
            ),
        ];
        Tester::new(NAME, json!([pattern]), pass, fail).test();
    }

    #[test]
    fn custom_property_pattern_scss() {
        let pass = [
            "a { color: var($foo-color); }",
            "a { color: var(bar.$baz); }",
            "a { --#{$bar}: 0; }",
            "a { color: var( #{$bar} ); }",
            "a { color: var( --#{$bar} ); }",
        ];
        Tester::new(NAME, json!(["/foo-.+/"]), pass, [])
            .with_custom_syntax("postcss-scss")
            .test();
    }

    #[test]
    fn custom_property_pattern_at_property() {
        let pattern = "/^[a-z]+(?:-[a-z]+)*$/";
        let pass = ["@property --foo-bar {}", "@property --foo, --bar {}"];
        let fail = [
            reject(
                "@property --Foo-bar {}",
                None,
                expected("--Foo-bar", pattern),
                (1, 11, 1, 20),
            ),
            reject(
                "@property --foo- {}",
                None,
                expected("--foo-", pattern),
                (1, 11, 1, 17),
            ),
            reject(
                "@property --foo, --Bar {}",
                None,
                expected("--Bar", pattern),
                (1, 18, 1, 23),
            ),
        ];
        Tester::new(NAME, json!([pattern]), pass, fail).test();
    }
}

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rules::Rule;
use crate::utils::ast::decl_value_raw;
use oxc_css_parser::Span;
use oxc_css_parser::ast::Declaration;
use serde_json::Value;

pub struct ColorNoInvalidHex {
    pub severity: Severity,
}

impl ColorNoInvalidHex {
    pub fn from_config(primary: &Value, severity: Severity) -> Option<Self> {
        if primary.as_bool() != Some(true) {
            return None;
        }
        Some(Self { severity })
    }
}

pub fn rejected_msg(hex: &str) -> String {
    format!("Invalid hex color \"{hex}\"")
}

impl Rule for ColorNoInvalidHex {
    fn name(&self) -> &'static str {
        "color-no-invalid-hex"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some((val_raw, val_span)) = decl_value_raw(ctx.source, decl) else {
            return;
        };

        if !val_raw.contains('#') {
            return;
        }

        if !is_standard_syntax_hex_color(val_raw.trim()) {
            return;
        }

        let nodes = parse_value_nodes(val_raw);
        self.walk_nodes(&nodes, val_raw, val_span.start, ctx);
    }
}

impl ColorNoInvalidHex {
    fn walk_nodes<'a>(
        &self,
        nodes: &[ValueNode],
        raw_source: &str,
        base_offset: u32,
        ctx: &mut LintContext<'a>,
    ) {
        for node in nodes {
            match node {
                ValueNode::Word { start, end } => {
                    let word = &raw_source[*start..*end];
                    if !is_standard_syntax_hex_color(word) {
                        continue;
                    }
                    if let Some(hex_value) = extract_hex_prefix(word) {
                        if !is_valid_hex(hex_value) {
                            let span = Span {
                                start: base_offset + *start as u32,
                                end: base_offset + (*start + hex_value.len()) as u32,
                            };
                            ctx.report(self.name(), span, rejected_msg(hex_value));
                        }
                    }
                }
                ValueNode::Function {
                    name_start,
                    name_end,
                    children,
                } => {
                    let func_name = &raw_source[*name_start..*name_end];
                    if func_name.to_ascii_lowercase().ends_with("url") {
                        continue;
                    }
                    self.walk_nodes(children, raw_source, base_offset, ctx);
                }
            }
        }
    }
}

fn is_standard_syntax_hex_color(s: &str) -> bool {
    !s.ends_with(']')
}

fn extract_hex_prefix(word: &str) -> Option<&str> {
    // Matches /^#[\da-z]+/i
    if !word.starts_with('#') {
        return None;
    }
    let alnum_len = word[1..]
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric())
        .count();
    if alnum_len == 0 {
        return None;
    }
    Some(&word[..1 + alnum_len])
}

fn is_valid_hex(hex: &str) -> bool {
    let Some(digits) = hex.strip_prefix('#') else {
        return false;
    };
    matches!(digits.len(), 3 | 4 | 6 | 8) && digits.bytes().all(|b| b.is_ascii_hexdigit())
}

#[derive(Debug)]
enum ValueNode {
    Word {
        start: usize,
        end: usize,
    },
    Function {
        name_start: usize,
        name_end: usize,
        children: Vec<ValueNode>,
    },
}

fn parse_value_nodes(s: &str) -> Vec<ValueNode> {
    let bytes = s.as_bytes();
    let (nodes, _) = parse_value_slice(s, bytes, 0, bytes.len(), false);
    nodes
}

fn parse_value_slice(
    s: &str,
    bytes: &[u8],
    mut i: usize,
    limit: usize,
    stop_at_close_paren: bool,
) -> (Vec<ValueNode>, usize) {
    let mut nodes = Vec::new();
    while i < limit {
        let b = bytes[i];

        if stop_at_close_paren && b == b')' {
            return (nodes, i + 1);
        }

        // Whitespace or dividers
        if b.is_ascii_whitespace() || matches!(b, b',' | b'/' | b':') {
            if b == b'/' && i + 1 < limit && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < limit && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(limit);
                continue;
            }
            if b == b'/' && i + 1 < limit && bytes[i + 1] == b'/' {
                i += 2;
                while i < limit && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            i += 1;
            continue;
        }

        // Strings
        if b == b'\'' || b == b'"' {
            let quote = b;
            i += 1;
            while i < limit {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(limit);
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

        // Parenthesized block without function name
        if b == b'(' {
            let full_start = i;
            let (children, next_i) = parse_value_slice(s, bytes, i + 1, limit, true);
            nodes.push(ValueNode::Function {
                name_start: full_start,
                name_end: full_start,
                children,
            });
            i = next_i;
            continue;
        }

        // Word or function
        let word_start = i;
        while i < limit {
            let c = bytes[i];
            if c == b'\\' {
                i = (i + 2).min(limit);
                continue;
            }
            if c.is_ascii_whitespace()
                || matches!(c, b',' | b'/' | b':' | b'(' | b')' | b'\'' | b'"')
            {
                break;
            }
            i += 1;
        }
        let word_end = i;

        if i < limit && bytes[i] == b'(' {
            let func_name = &s[word_start..word_end];
            if func_name.to_ascii_lowercase().ends_with("url") {
                let next_i = skip_url_args(bytes, i + 1, limit);
                nodes.push(ValueNode::Function {
                    name_start: word_start,
                    name_end: word_end,
                    children: Vec::new(),
                });
                i = next_i;
            } else {
                let (children, next_i) = parse_value_slice(s, bytes, i + 1, limit, true);
                nodes.push(ValueNode::Function {
                    name_start: word_start,
                    name_end: word_end,
                    children,
                });
                i = next_i;
            }
        } else if word_end > word_start {
            nodes.push(ValueNode::Word {
                start: word_start,
                end: word_end,
            });
        } else {
            i += 1;
        }
    }
    (nodes, i)
}

fn skip_url_args(bytes: &[u8], mut i: usize, limit: usize) -> usize {
    while i < limit && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < limit && (bytes[i] == b'\'' || bytes[i] == b'"') {
        let quote = bytes[i];
        i += 1;
        while i < limit {
            if bytes[i] == b'\\' {
                i = (i + 2).min(limit);
                continue;
            }
            if bytes[i] == quote {
                i += 1;
                break;
            }
            i += 1;
        }
        while i < limit && bytes[i] != b')' {
            i += 1;
        }
        return if i < limit && bytes[i] == b')' {
            i + 1
        } else {
            i
        };
    }

    while i < limit && bytes[i] != b')' {
        if bytes[i] == b'\\' {
            i = (i + 2).min(limit);
            continue;
        }
        i += 1;
    }
    if i < limit && bytes[i] == b')' {
        i + 1
    } else {
        i
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_message};

    const NAME: &str = "color-no-invalid-hex";

    fn rejected(hex: &str) -> String {
        format!("Invalid hex color \"{hex}\"")
    }

    #[test]
    fn default() {
        let pass = &[
            "a { color: pink; }",
            "a { color: #000; }",
            "a { something: #000, #fff, #ababab; }",
            "a { color: #0000ffcc; }",
            "a { color:#00fc; }",
            "a { padding: 000; }",
            "a::before { content: \"#ababa\"; }",
            "a { background-image: svg-load('x.svg', fill=url(#a)); }",
            "a { background-image: url(#a); }",
            "a { background-image: url(x.svg#a); }",
            "@font-face {\nfont-family: dashicons;\nsrc: url(data:application/font-woff;charset=utf-8;base64, ABCDEF==) format(\"woff\"),\nurl(../fonts/dashicons.ttf) format(\"truetype\"),\nurl(../fonts/dashicons.svg#dashicons) format(\"svg\");\nfont-weight: normal;\nfont-style: normal;\n}",
        ];

        let fail = &[
            reject(
                "a { color: #ababa; }",
                None,
                rejected("#ababa"),
                (1, 12, 1, 18),
            ),
            reject(
                "a { something: #00, #fff, #ababab; }",
                None,
                rejected("#00"),
                (1, 16, 1, 19),
            ),
            reject(
                "a { something: #000, #fff1az, #ababab; }",
                None,
                rejected("#fff1az"),
                (1, 22, 1, 29),
            ),
            reject(
                "a { something:#000,#fff,#12345aa; }",
                None,
                rejected("#12345aa"),
                (1, 25, 1, 33),
            ),
        ];

        Tester::new(NAME, json!([true]), pass, fail).test();
    }

    #[test]
    fn less() {
        let pass = &["a { color: #colors[somecolor]; }"];

        Tester::new(NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-less")
            .test();
    }

    #[test]
    fn scss() {
        let pass = &["a { border-#$side: 0; }", "a { box-sizing: #$type-box; }"];

        Tester::new(NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-scss")
            .test();
    }

    #[test]
    #[ignore = "naive-css-in-js is not supported"]
    fn css_in_js() {
        let pass = &["css` color: #ffff; `;"];
        let fail = &[reject_message(
            "css` color: #fffff; `;",
            None,
            rejected("#fffff"),
        )];

        Tester::new(NAME, json!([true]), pass, fail)
            .with_custom_syntax("naive-css-in-js")
            .test();
    }
}

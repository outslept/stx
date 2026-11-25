use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rules::Rule;
use crate::utils::ast::decl_value_raw;
use oxc_css_parser::Span;
use oxc_css_parser::ast::Declaration;
use regex::Regex;
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
enum StringOrRegex {
    Exact(String),
    Regex(Regex),
}

impl StringOrRegex {
    fn parse(s: &str) -> Option<Self> {
        if s.starts_with('/') && s.len() >= 2 {
            if let Some(last_slash) = s[1..].rfind('/') {
                let pattern = &s[1..=last_slash];
                let flags = &s[last_slash + 2..];
                let full_pattern = if flags.contains('i') {
                    format!("(?i){pattern}")
                } else {
                    pattern.to_string()
                };
                if let Ok(re) = Regex::new(&full_pattern) {
                    return Some(Self::Regex(re));
                }
            }
        }
        Some(Self::Exact(s.to_string()))
    }

    fn is_match(&self, candidate: &str) -> bool {
        match self {
            Self::Exact(s) => s == candidate,
            Self::Regex(re) => re.is_match(candidate),
        }
    }
}

pub struct ColorNoHex {
    ignore_functions: Vec<StringOrRegex>,
    pub severity: Severity,
}

impl ColorNoHex {
    pub fn from_config(
        primary: &Value,
        secondary: Option<&Map<String, Value>>,
        severity: Severity,
    ) -> Option<Self> {
        if primary.as_bool() != Some(true) {
            return None;
        }

        let mut ignore_functions = Vec::new();
        if let Some(sec) = secondary {
            if let Some(arr) = sec.get("ignoreFunctions").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if let Some(matcher) = StringOrRegex::parse(s) {
                            ignore_functions.push(matcher);
                        }
                    }
                }
            }
        }

        Some(Self {
            ignore_functions,
            severity,
        })
    }

    fn is_ignored_function(&self, func_name: &str) -> bool {
        self.ignore_functions.iter().any(|m| m.is_match(func_name))
    }
}

pub fn rejected_msg(hex: &str) -> String {
    format!("Disallowed hex color \"{hex}\"")
}

impl Rule for ColorNoHex {
    fn name(&self) -> &'static str {
        "color-no-hex"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some((val_raw, val_span)) = decl_value_raw(ctx.source, decl) else {
            return;
        };

        if !val_raw.contains('#') {
            return;
        }

        let nodes = parse_value_nodes(val_raw);
        self.walk_nodes(&nodes, val_raw, val_span.start, ctx);
    }
}

impl ColorNoHex {
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
                    if is_hex_color(word) {
                        let span = Span {
                            start: base_offset + *start as u32,
                            end: base_offset + *end as u32,
                        };
                        ctx.report(self.name(), span, rejected_msg(word));
                    }
                }
                ValueNode::Function {
                    name_start,
                    name_end,
                    children,
                } => {
                    let func_name = &raw_source[*name_start..*name_end];
                    if func_name.eq_ignore_ascii_case("url") {
                        continue;
                    }
                    if self.is_ignored_function(func_name) {
                        continue;
                    }
                    self.walk_nodes(children, raw_source, base_offset, ctx);
                }
            }
        }
    }
}

fn is_hex_color(word: &str) -> bool {
    // Matches /^#[\da-z]+$/i
    word.starts_with('#') && word.len() > 1 && word[1..].bytes().all(|b| b.is_ascii_alphanumeric())
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
            if func_name.eq_ignore_ascii_case("url") {
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

    use crate::tester::{Tester, reject, reject_message, reject_messages};

    const NAME: &str = "color-no-hex";

    fn rejected(hex: &str) -> String {
        format!("Disallowed hex color \"{hex}\"")
    }

    #[test]
    fn default() {
        let pass = &[
            "a { color: pink; }",
            "a { color: rgba(0, 0, 0, 0); }",
            "a { something: black, white, gray; }",
            "a { padding: 000; }",
            "a::before { content: \"#ababa\"; }",
            "a { border-#$side: 0; }",
            "a { box-sizing: #$type-box; }",
            "@font-face {\nfont-family: dashicons;\nsrc: url(data:application/font-woff;charset=utf-8;base64, ABCDEF==) format(\"woff\"),\nurl(../fonts/dashicons.ttf) format(\"truetype\"),\nurl(../fonts/dashicons.svg#dashicons) format(\"svg\");\nfont-weight: normal;\nfont-style: normal;\n}",
        ];

        let fail = &[
            reject(
                "a { color: #12345; }",
                None,
                rejected("#12345"),
                (1, 12, 1, 18),
            ),
            reject(
                "a { color: #123456a; }",
                None,
                rejected("#123456a"),
                (1, 12, 1, 20),
            ),
            reject(
                "a { color: #cccccc; }",
                None,
                rejected("#cccccc"),
                (1, 12, 1, 19),
            ),
            reject(
                "a { something: #00c, red, white; }",
                None,
                rejected("#00c"),
                (1, 16, 1, 20),
            ),
            reject(
                "a { something: black, #fff1a1, rgb(250, 250, 0); }",
                None,
                rejected("#fff1a1"),
                (1, 23, 1, 30),
            ),
            reject(
                "a { something:black,white,#12345a; }",
                None,
                rejected("#12345a"),
                (1, 27, 1, 34),
            ),
            reject(
                "a { color: #ffff; }",
                None,
                rejected("#ffff"),
                (1, 12, 1, 17),
            ),
            reject(
                "a { color: #ffffffaa; }",
                None,
                rejected("#ffffffaa"),
                (1, 12, 1, 21),
            ),
        ];

        Tester::new(NAME, json!([true]), pass, fail).test();
    }

    #[test]
    #[ignore = "naive-css-in-js is not supported"]
    fn css_in_js() {
        let pass = &[
            "css` color: pink; `;",
            "css` color: rgba(0, 0, 0, 0); `;",
            "css` content: \"#abcdef\"; `;",
            "css` background: url(#aabbcc); `;",
            "css` color: red /* #ff0000 */; `;",
            "css` color: linear-gradient(green, rgb(123, 123, 123), hsl(24, 70%, 80%)); `;",
        ];

        let fail = &[
            reject_message("css` color: #abcdef `;", None, rejected("#abcdef")),
            reject_messages(
                "css` background-color: linear-gradient(#aaa, #ffff, #01234567); `;",
                None,
                vec![rejected("#aaa"), rejected("#ffff"), rejected("#01234567")],
            ),
        ];

        Tester::new(NAME, json!([true]), pass, fail)
            .with_custom_syntax("naive-css-in-js")
            .test();
    }

    #[test]
    fn ignore_functions() {
        let pass = &[
            "a { color: var(--foo, #fff); }",
            "a { color: --foo(#fff); }",
        ];

        let fail = &[reject(
            "a { color: #ffffff; }",
            None,
            rejected("#ffffff"),
            (1, 12, 1, 19),
        )];

        Tester::new(
            NAME,
            json!([true, { "ignoreFunctions": ["var", "/^--/"] }]),
            pass,
            fail,
        )
        .test();
    }
}

use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::Stylesheet};

pub struct NoIrregularWhitespace;

fn is_irregular_whitespace(ch: char) -> bool {
    matches!(
        ch,
        '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{00A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'
            ..='\u{200B}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

fn is_grid_string_checked_prop(prop: &str) -> bool {
    prop.eq_ignore_ascii_case("grid")
        || prop.eq_ignore_ascii_case("grid-template")
        || prop.eq_ignore_ascii_case("grid-template-areas")
}

impl Rule for NoIrregularWhitespace {
    fn name(&self) -> &'static str {
        "no-irregular-whitespace"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let src = ctx.source;
        let bytes = src.as_bytes();
        let len = bytes.len();
        let mut i = 0usize;

        let mut last_ident = String::new();
        let mut in_decl_value = false;
        let mut check_strings_in_value = false;

        while i < len {
            // Skip block comments `/* ... */`
            if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                continue;
            }

            // Handle quoted strings `'...'` and `"..."`
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                i += 1;
                if check_strings_in_value {
                    while i < len {
                        if bytes[i] == b'\\' {
                            i = (i + 2).min(len);
                            continue;
                        }
                        if bytes[i] == quote {
                            i += 1;
                            break;
                        }
                        let ch = src[i..].chars().next().unwrap();
                        if is_irregular_whitespace(ch) {
                            let start = i as u32;
                            i += ch.len_utf8();
                            while i < len && bytes[i] != quote {
                                let next_ch = src[i..].chars().next().unwrap();
                                if is_irregular_whitespace(next_ch) {
                                    i += next_ch.len_utf8();
                                } else {
                                    break;
                                }
                            }
                            let end = i as u32;
                            ctx.report(
                                self.name(),
                                Span { start, end },
                                "Irregular whitespace".to_string(),
                            );
                            continue;
                        }
                        i += ch.len_utf8();
                    }
                } else {
                    while i < len {
                        if bytes[i] == b'\\' {
                            i = (i + 2).min(len);
                            continue;
                        }
                        if bytes[i] == quote {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                }
                continue;
            }

            let ch = src[i..].chars().next().unwrap();
            if is_irregular_whitespace(ch) {
                let start = i as u32;
                i += ch.len_utf8();
                while i < len {
                    let next_ch = src[i..].chars().next().unwrap();
                    if is_irregular_whitespace(next_ch) {
                        i += next_ch.len_utf8();
                    } else {
                        break;
                    }
                }
                let end = i as u32;
                ctx.report(
                    self.name(),
                    Span { start, end },
                    "Irregular whitespace".to_string(),
                );
                continue;
            }

            match ch {
                '{' | '}' | ';' => {
                    last_ident.clear();
                    in_decl_value = false;
                    check_strings_in_value = false;
                }
                ':' if !in_decl_value => {
                    in_decl_value = true;
                    check_strings_in_value = is_grid_string_checked_prop(last_ident.trim());
                }
                _ if !in_decl_value => {
                    if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                        last_ident.push(ch);
                    } else if !ch.is_ascii_whitespace() {
                        last_ident.clear();
                    }
                }
                _ => {}
            }

            i += ch.len_utf8();
        }
    }
}

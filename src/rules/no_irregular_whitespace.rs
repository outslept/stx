use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::Stylesheet};

crate::define_stx_lint! {
    /// Report irregular whitespace such as non-breaking and zero-width spaces.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/no-irregular-whitespace>
    ///
    /// Enable with `true`; there are no rule-specific options. Block comments
    /// and quoted strings are skipped, except that strings in `grid`,
    /// `grid-template`, and `grid-template-areas` values are checked.
    ///
    /// The space between `main` and `sidebar` below is a non-breaking space (U+00A0):
    ///
    /// ```css
    /// .layout { grid-template-areas: "main sidebar"; }
    /// ```
    "no-irregular-whitespace", correctness, recommended = true;
    pub struct NoIrregularWhitespace;
}

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
        Self::NAME
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

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    const REJECTED: &str = "Irregular whitespace";

    #[test]
    fn default_config() {
        let pass = [
            " ",
            "\n",
            "/* Irregular\u{00A0}whitespace\nin\u{00A0}multi line comments\nare\u{00A0}allowed */",
            "/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0}b, /* stylelint-disable-next-line no-irregular-whitespace */\na\u{00A0}b\n/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0}\n{}",
            "/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0}@media\n/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0} /* stylelint-disable-next-line no-irregular-whitespace */\nonly\u{00A0}screen\n{}",
            ".foo {\n\t/* stylelint-disable-next-line no-irregular-whitespace */\n\tpadding\u{00A0}: /* stylelint-disable-next-line no-irregular-whitespace */\n\t\u{00A0} /* stylelint-disable-next-line no-irregular-whitespace */\n\t1px\u{00A0}2px\n\t/* stylelint-disable-next-line no-irregular-whitespace */\n\tcalc(5px\u{00A0}+ 6px);\n}",
            "a { content: '\u{00A0}'; }",
            "a { content: \"foo \t \n  bar\"; }",
            "@charset \"\u{00A0}\"",
            "a[id=\"\u{00A0}\"] {}",
            "a { animation-name: \"foo\u{00A0}\" }",
            "a { animation-name: 'ba\u{00A0}r' }",
            "a { font-family: 'Times\u{0085}New\u{00A0}\u{00A0}Roman', serif; }",
            "@font-feature-values \"foo\u{00A0}bar\"",
            "a[title=\"irregular\u{000B}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{000C}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{00A0}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{0085}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{1680}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{180E}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{FEFF}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2000}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2001}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2002}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2003}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2004}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2005}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2006}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2007}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2008}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2009}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{200A}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{200B}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2028}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{2029}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{202F}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{205F}whitespace\"] { color: pink; }",
            "a[title=\"irregular\u{3000}whitespace\"] { color: pink; }",
            ":lang(\"en\u{00A0}us \")",
            ":not([data-id=\"a\u{00A0}b\"])",
            "a[data-x='foo \u{00A0}  bar'] {}",
            ":is(.class[attr=\"\u{00A0}a\"])",
            ":is(input[type=\"checkbox\u{00A0}\"])",
            "a[class='\u{00A0}foo'] {}",
            "a[id='f\u{00A0}oo'] {}",
            "[data-y=\"foo\tbar\"] {}",
            "input[placeholder=\"foo\u{2009}bar\"] {}",
            ":nth-child(\" 2n+ 1\u{00A0}\")",
            "a/*foo\u{00A0}bar*/[attr=\"test\"] {}",
            "a { /* comment\u{00A0}*/ color: red; }",
            "a { grid-template-areas: /* \"comment\u{00A0}\" */ none /* \"\u{00A0}comment \" */; }",
            "a { grid-template-areas: /* comment\u{00A0} */ none /* \u{00A0}comment */; }",
            "a { cursor: url(\"pointer.cur\u{00A0}\"), pointer; }",
            "@counter-style custom-list { symbols: \"-\u{00A0}\" \"◦\" \"‣\"; }",
        ];
        let fail = [
            reject("@\u{00A0}charset \"utf-8\"", None, REJECTED, (1, 2, 1, 3)),
            reject("@charset\u{00A0} \"utf-8\"", None, REJECTED, (1, 9, 1, 10)),
            reject(
                ".firstClass\u{00A0}.secondClass { color: pink; }",
                None,
                REJECTED,
                (1, 12, 1, 13),
            ),
            reject(
                ".firstClass .secondClass \u{00A0}{ color: pink; }",
                None,
                REJECTED,
                (1, 26, 1, 27),
            ),
            reject("margin: 1rem\u{00A0}2rem;", None, REJECTED, (1, 13, 1, 14)),
            reject("margin: 1rem 2rem\u{00A0};", None, REJECTED, (1, 18, 1, 19)),
            reject("$variable\u{00A0}: 5rem;", None, REJECTED, (1, 10, 1, 11)),
            reject("@media screen {\u{000C}}", None, REJECTED, (1, 16, 1, 17)),
            reject(".a {\u{000C}}", None, REJECTED, (1, 5, 1, 6)),
            reject(".a {\u{000C}\u{000C}}", None, REJECTED, (1, 5, 1, 7)),
            reject_warnings(
                "/* a comment */\n\u{00A0}b, /* a comment */\na\u{00A0}b\n/* a comment */\n\u{00A0}\n{}",
                None,
                vec![
                    (REJECTED.to_string(), (2, 1, 2, 2)),
                    (REJECTED.to_string(), (3, 2, 3, 3)),
                    (REJECTED.to_string(), (5, 1, 5, 2)),
                ],
            ),
            reject_warnings(
                "/* a comment */\n\u{00A0}@media\n/* a comment */\n\u{00A0} /* a comment */\nonly\u{00A0}screen\n{}",
                None,
                vec![
                    (REJECTED.to_string(), (2, 1, 2, 2)),
                    (REJECTED.to_string(), (4, 1, 4, 2)),
                    (REJECTED.to_string(), (5, 5, 5, 6)),
                ],
            ),
            reject_warnings(
                ".foo {\n\t/* a comment */\n\tpadding\u{00A0}: /* a comment */\n\t\u{00A0} /* a comment */\n\t1px\u{00A0}2px\n\t/* a comment */\n\tcalc(5px\u{00A0}+ 6px);\n}",
                None,
                vec![
                    (REJECTED.to_string(), (3, 9, 3, 10)),
                    (REJECTED.to_string(), (4, 2, 4, 3)),
                    (REJECTED.to_string(), (5, 5, 5, 6)),
                    (REJECTED.to_string(), (7, 10, 7, 11)),
                ],
            ),
            reject_warnings(
                "a {\n\tgrid-template:\n\t\t\"a a\u{00A0}a\" 40px\n\t\t\"b c c\" 40px\n\t\t\"b\u{00A0}c c\" 40px / 1fr 1fr 1fr;\n}",
                None,
                vec![
                    (REJECTED.to_string(), (3, 7, 3, 8)),
                    (REJECTED.to_string(), (5, 5, 5, 6)),
                ],
            ),
            reject(
                "a { grid-template-areas: \"a\u{00A0}a\" \"b b\"; }",
                None,
                REJECTED,
                (1, 28, 1, 29),
            ),
            reject(
                "a { grid: \"foo\u{00A0}\" 1fr /* \u{00A0}comment */ \"bar\" 1fr; }",
                None,
                REJECTED,
                (1, 15, 1, 16),
            ),
        ];

        Tester::new("no-irregular-whitespace", json!([true]), pass, fail).test();
    }
}

//! Port of Stylelint's `assignDisabledRanges` + `isDisabledOnLine`.

use crate::{line_index::LineIndex, utils::is_configuration_comment};
use oxc_css_parser::token::{Comment, CommentKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Range {
    start: usize,
    end: Option<usize>,
}

/// Disabled ranges keyed by rule name; `None` is the `all` entry and is always first.
#[derive(Debug, Default)]
pub struct DisableDirectives {
    ranges: Vec<(Option<String>, Vec<Range>)>,
}

impl DisableDirectives {
    pub fn parse(comments: &[Comment<'_>], source: &str, line_index: &LineIndex) -> Self {
        let mut this = Self {
            ranges: vec![(None, Vec::new())],
        };
        let line = |offset: u32| line_index.line_col(source, offset).0;
        let mut i = 0;
        while i < comments.len() {
            let c = &comments[i];
            let mut text = c.content.trim().to_string();
            let start_line = line(c.span.start);
            let mut end_line = line(c.span.end.saturating_sub(1));
            i += 1;
            // Merge a `//` directive with the following `//` lines of its multi-line description.
            if c.kind == CommentKind::Line
                && is_configuration_comment(&text)
                && let Some(next) = comments.get(i)
                && next.kind == CommentKind::Line
                && adjacent(source, c, next)
                && (text.contains("--") || next.content.trim().starts_with("--"))
            {
                let mut prev = c;
                while let Some(cur) = comments.get(i) {
                    let cur_line = line(cur.span.end.saturating_sub(1));
                    if cur.kind != CommentKind::Line
                        || is_configuration_comment(cur.content)
                        || !adjacent(source, prev, cur)
                        || cur_line != end_line + 1
                    {
                        break;
                    }
                    text.push('\n');
                    text.push_str(cur.content.trim());
                    end_line = cur_line;
                    prev = cur;
                    i += 1;
                }
            }
            this.check_comment(&text, start_line, end_line);
        }
        this
    }

    fn check_comment(&mut self, text: &str, start_line: usize, end_line: usize) {
        let Some((command, rules)) = split_command(text) else {
            return;
        };
        let rules = rule_list(rules);
        match command {
            "disable-line" => rules
                .into_iter()
                .for_each(|r| self.disable_line(start_line, r)),
            "disable-next-line" => rules
                .into_iter()
                .for_each(|r| self.disable_line(end_line + 1, r)),
            "disable" => rules.into_iter().for_each(|r| self.disable(start_line, r)),
            _ => rules.into_iter().for_each(|r| self.enable(end_line, r)),
        }
    }

    // shortcut: Stylelint throws on invalid sequences ("already disabled", "not disabled");
    // STX ignores the offending command. Report it once disable-reporting is implemented.
    fn disable_line(&mut self, line: usize, rule: Option<&str>) {
        if self.is_disabled(None) {
            return;
        }
        match rule {
            None => {
                for idx in 0..self.ranges.len() {
                    if !last_open(&self.ranges[idx].1) {
                        self.ranges[idx].1.push(Range {
                            start: line,
                            end: Some(line),
                        });
                    }
                }
            }
            Some(name) if !self.is_disabled(Some(name)) => {
                self.entry(name).push(Range {
                    start: line,
                    end: Some(line),
                });
            }
            Some(_) => {}
        }
    }

    fn disable(&mut self, line: usize, rule: Option<&str>) {
        if self.is_disabled(rule) {
            return;
        }
        match rule {
            None => self.ranges.iter_mut().for_each(|(_, r)| {
                r.push(Range {
                    start: line,
                    end: None,
                })
            }),
            Some(name) => self.entry(name).push(Range {
                start: line,
                end: None,
            }),
        }
    }

    fn enable(&mut self, line: usize, rule: Option<&str>) {
        let Some(name) = rule else {
            for (_, ranges) in &mut self.ranges {
                if let Some(last) = ranges.last_mut().filter(|r| r.end.is_none()) {
                    last.end = Some(line);
                }
            }
            return;
        };
        if self.is_disabled(None) && self.position(name).is_none() {
            let copy = self.ranges[0].1.clone();
            self.ranges.push((Some(name.to_string()), copy));
        } else if !self.is_disabled(Some(name)) {
            return;
        }
        if let Some(last) = self.entry(name).last_mut() {
            last.end = Some(line);
        }
    }

    fn position(&self, name: &str) -> Option<usize> {
        self.ranges
            .iter()
            .position(|(n, _)| n.as_deref() == Some(name))
    }

    /// Ranges of `name`, created from a copy of the `all` ranges on first use.
    fn entry(&mut self, name: &str) -> &mut Vec<Range> {
        let idx = self.position(name).unwrap_or_else(|| {
            let copy = self.ranges[0].1.clone();
            self.ranges.push((Some(name.to_string()), copy));
            self.ranges.len() - 1
        });
        &mut self.ranges[idx].1
    }

    fn is_disabled(&self, rule: Option<&str>) -> bool {
        let ranges = match rule {
            None => Some(&self.ranges[0].1),
            Some(name) => self.position(name).map(|i| &self.ranges[i].1),
        };
        ranges.is_some_and(|r| last_open(r))
    }

    pub fn is_suppressed(&self, rule_name: &str, line: usize) -> bool {
        let ranges = self
            .position(rule_name)
            .map_or(&self.ranges[0].1, |i| &self.ranges[i].1);
        ranges
            .iter()
            .any(|r| r.start <= line && r.end.is_none_or(|end| end >= line))
    }
}

fn last_open(ranges: &[Range]) -> bool {
    ranges.last().is_some_and(|r| r.end.is_none())
}

fn adjacent(source: &str, a: &Comment<'_>, b: &Comment<'_>) -> bool {
    source
        .get(a.span.end as usize..b.span.start as usize)
        .is_some_and(|gap| gap.trim().is_empty())
}

fn split_command(text: &str) -> Option<(&'static str, &str)> {
    let rest = text
        .strip_prefix("stylelint-")
        .or_else(|| text.strip_prefix("stx-"))?;
    ["disable-next-line", "disable-line", "disable", "enable"]
        .into_iter()
        .find_map(|cmd| {
            let after = rest.strip_prefix(cmd)?;
            (after.is_empty() || after.starts_with(char::is_whitespace)).then_some((cmd, after))
        })
}

/// Rule names before the `--` description; an empty list means all rules (`None`).
fn rule_list(rest: &str) -> Vec<Option<&str>> {
    let list = rest.split("--").next().unwrap_or("");
    let rules: Vec<_> = list
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| (s != "all").then_some(s))
        .collect();
    if rules.is_empty() { vec![None] } else { rules }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_css_parser::{Allocator, ParserBuilder, Syntax, ast::Stylesheet};

    type Expected<'a> = &'a [(&'a str, &'a [(usize, Option<usize>)])];

    fn parse(source: &str, syntax: Syntax) -> DisableDirectives {
        let alloc = Allocator::default();
        let mut parser = ParserBuilder::new(&alloc, source)
            .syntax(syntax)
            .comments()
            .build();
        let _ = parser.parse::<Stylesheet>();
        DisableDirectives::parse(parser.comments(), source, &LineIndex::new(source))
    }

    fn check_with(syntax: Syntax, source: &str, expected: Expected<'_>) {
        let actual: Vec<_> = parse(source, syntax)
            .ranges
            .into_iter()
            .map(|(name, ranges)| {
                let name = name.unwrap_or_else(|| "all".into());
                (
                    name,
                    ranges
                        .into_iter()
                        .map(|r| (r.start, r.end))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        let expected: Vec<_> = expected
            .iter()
            .map(|(name, ranges)| (name.to_string(), ranges.to_vec()))
            .collect();
        assert_eq!(actual, expected, "source:\n{source}");
    }

    fn check(source: &str, expected: Expected<'_>) {
        check_with(Syntax::Css, source, expected);
    }

    #[test]
    fn whole_file_ranges() {
        check("a {}", &[("all", &[])]);
        check("/* stylelint-disable */\na {}", &[("all", &[(1, None)])]);
        check(
            "a {}\n/* stylelint-disable */\nb {}\n/* stylelint-enable */\n.foo {}",
            &[("all", &[(2, Some(4))])],
        );
        check(
            "a {}\n/* stylelint-disable */\nb {}\n/* stylelint-enable */\n.foo {}\n/* stylelint-disable */\nb {}\n/* stylelint-enable */\n.foo {}",
            &[("all", &[(2, Some(4)), (6, Some(8))])],
        );
    }

    #[test]
    fn rule_ranges() {
        check(
            "/* stylelint-disable foo-bar */\na {}",
            &[("all", &[]), ("foo-bar", &[(1, None)])],
        );
        check(
            "a {}\n/* stylelint-disable foo-bar */\nb {}\n/* stylelint-enable */\n.foo {}\n/* stylelint-disable foo-bar,baz-maz */\nb {}\n/* stylelint-enable */\n.foo {}",
            &[
                ("all", &[]),
                ("foo-bar", &[(2, Some(4)), (6, Some(8))]),
                ("baz-maz", &[(6, Some(8))]),
            ],
        );
        check(
            "/* stylelint-disable foo-bar, hoo-hah,\n\tslime */\nb {}",
            &[
                ("all", &[]),
                ("foo-bar", &[(1, None)]),
                ("hoo-hah", &[(1, None)]),
                ("slime", &[(1, None)]),
            ],
        );
    }

    #[test]
    fn disable_line() {
        check(
            "a {} /* stylelint-disable-line */",
            &[("all", &[(1, Some(1))])],
        );
        check(
            "a {} /* stylelint-disable-line block-no-empty */",
            &[("all", &[]), ("block-no-empty", &[(1, Some(1))])],
        );
        check(
            "b {}\n\na {} /* stylelint-disable-line block-no-empty, blergh */",
            &[
                ("all", &[]),
                ("block-no-empty", &[(3, Some(3))]),
                ("blergh", &[(3, Some(3))]),
            ],
        );
        check(
            ".foo{\n\t/* stylelint-disable-line */\n\ta/* stylelint-disable-line */\n\t:/* stylelint-disable-line */\n\tb/* stylelint-disable-line */\n\tc/* stylelint-disable-line */\n\t/* stylelint-disable-line */d\n\te(/* stylelint-disable-line */)\n\t;/* stylelint-disable-line*/\n}",
            &[(
                "all",
                &[
                    (2, Some(2)),
                    (3, Some(3)),
                    (4, Some(4)),
                    (5, Some(5)),
                    (6, Some(6)),
                    (7, Some(7)),
                    (8, Some(8)),
                    (9, Some(9)),
                ],
            )],
        );
        check(
            "/* offset by one line */\na,/* stylelint-disable-line */\nb/* stylelint-disable-line */,\nc,/* stylelint-disable-line */\nlong-selector/* stylelint-disable-line */,\n/* stylelint-disable-line */long-selector,\n:is(/* stylelint-disable-line */d),\ne/* stylelint-disable-line */ {}",
            &[(
                "all",
                &[
                    (2, Some(2)),
                    (3, Some(3)),
                    (4, Some(4)),
                    (5, Some(5)),
                    (6, Some(6)),
                    (7, Some(7)),
                    (8, Some(8)),
                ],
            )],
        );
        check(
            "/* offset by one line */\n@a/* stylelint-disable-line */\n/* stylelint-disable-line*/tty,\ntv/* stylelint-disable-line*/,\n(min-width: 20px/* stylelint-disable-line*/),\nscreen,/* stylelint-disable-line*/\nprint/* stylelint-disable-line*/ {}",
            &[(
                "all",
                &[
                    (2, Some(2)),
                    (3, Some(3)),
                    (4, Some(4)),
                    (5, Some(5)),
                    (6, Some(6)),
                    (7, Some(7)),
                ],
            )],
        );
    }

    #[test]
    fn disable_next_line() {
        check(
            "/* stylelint-disable-next-line */\na {} ",
            &[("all", &[(2, Some(2))])],
        );
        check(
            "/* stylelint-disable-next-line block-no-empty */\na {}",
            &[("all", &[]), ("block-no-empty", &[(2, Some(2))])],
        );
        check(
            "\nb {}\n\n/* stylelint-disable-next-line block-no-empty, blergh */\na {}",
            &[
                ("all", &[]),
                ("block-no-empty", &[(5, Some(5))]),
                ("blergh", &[(5, Some(5))]),
            ],
        );
        check(
            "/* stylelint-disable-next-line\n   block-no-empty */\na {}",
            &[("all", &[]), ("block-no-empty", &[(3, Some(3))])],
        );
    }

    #[test]
    fn nested_ranges() {
        check(
            "/* stylelint-disable foo */\n/* stylelint-disable bar */\n/* stylelint-disable baz, hop */\n/* stylelint-enable bar */\n/* stylelint-enable foo, hop */\n/* stylelint-enable baz */",
            &[
                ("all", &[]),
                ("foo", &[(1, Some(5))]),
                ("bar", &[(2, Some(4))]),
                ("baz", &[(3, Some(6))]),
                ("hop", &[(3, Some(5))]),
            ],
        );
        check(
            "/* stylelint-disable */\n/* stylelint-enable bar */\n/* stylelint-disable bar */\n/* stylelint-enable */",
            &[
                ("all", &[(1, Some(4))]),
                ("bar", &[(1, Some(2)), (3, Some(4))]),
            ],
        );
        check(
            "/* stylelint-disable foo */\n/* stylelint-disable bar, baz */\n/* stylelint-enable */",
            &[
                ("all", &[]),
                ("foo", &[(1, Some(3))]),
                ("bar", &[(2, Some(3))]),
                ("baz", &[(2, Some(3))]),
            ],
        );
        check(
            "/* stylelint-disable */\n/* stylelint-enable foo */\n/* stylelint-enable */\n/* stylelint-disable bar */",
            &[
                ("all", &[(1, Some(3))]),
                ("foo", &[(1, Some(2))]),
                ("bar", &[(1, Some(3)), (4, None)]),
            ],
        );
        check(
            "/* stylelint-disable */\n/* stylelint-enable bar */\n/* stylelint-enable foo */\n/* stylelint-disable foo */\n/* stylelint-enable */",
            &[
                ("all", &[(1, Some(5))]),
                ("bar", &[(1, Some(2))]),
                ("foo", &[(1, Some(3)), (4, Some(5))]),
            ],
        );
    }

    #[test]
    fn invalid_sequences_are_ignored() {
        check(
            "a {} /* stylelint-disable-line block-no-empty */\n/* stylelint-enable */",
            &[("all", &[]), ("block-no-empty", &[(1, Some(1))])],
        );
        check(
            "/* stylelint-disable */\na {} /* stylelint-disable-line */",
            &[("all", &[(1, None)])],
        );
        check(
            "/* stylelint-disable */\na {} /* stylelint-disable-line foo */",
            &[("all", &[(1, None)])],
        );
        check(
            "/* stylelint-disable foo */\na {} /* stylelint-disable-line foo */",
            &[("all", &[]), ("foo", &[(1, None)])],
        );
        check(
            "/* stylelint-disable */ /* stylelint-disable */",
            &[("all", &[(1, None)])],
        );
        check(
            "/* stylelint-disable foo */ /* stylelint-disable foo*/",
            &[("all", &[]), ("foo", &[(1, None)])],
        );
        check("/* stylelint-enable */", &[("all", &[])]);
        check("/* stylelint-enable foo */", &[("all", &[])]);
    }

    #[test]
    fn descriptions() {
        check(
            "/* stylelint-disable -- Description */\na {}",
            &[("all", &[(1, None)])],
        );
        check(
            "a {}\n/* stylelint-disable -- Description */\nb {}\n/* stylelint-enable -- Description */\n.foo {}",
            &[("all", &[(2, Some(4))])],
        );
        check(
            "/* stylelint-disable foo-bar -- Description */\na {}",
            &[("all", &[]), ("foo-bar", &[(1, None)])],
        );
        check(
            "/* stylelint-disable foo-bar, hoo-hah,\n\tslime -- Description */\nb {}",
            &[
                ("all", &[]),
                ("foo-bar", &[(1, None)]),
                ("hoo-hah", &[(1, None)]),
                ("slime", &[(1, None)]),
            ],
        );
    }

    #[test]
    fn double_slash_comments() {
        for syntax in [Syntax::Scss, Syntax::Less] {
            check_with(
                syntax,
                "a {\n\tcolor: pink !important; // stylelint-disable-line declaration-no-important\n}",
                &[("all", &[]), ("declaration-no-important", &[(2, Some(2))])],
            );
            check_with(
                syntax,
                "a {\n\tcolor: pink !important; // stylelint-disable-line declaration-no-important -- Description\n}",
                &[("all", &[]), ("declaration-no-important", &[(2, Some(2))])],
            );
            check_with(
                syntax,
                "a {\n\t// stylelint-disable-next-line declaration-no-important\n\t// --\n\t// Long-winded description\n\tcolor: pink !important;\n}",
                &[("all", &[]), ("declaration-no-important", &[(5, Some(5))])],
            );
        }
        let scss = |source, expected| check_with(Syntax::Scss, source, expected);
        scss(
            "a {\n\t// stylelint-disable declaration-no-important\n\n\t// Unrelated\n\tcolor: pink !important;\n}",
            &[("all", &[]), ("declaration-no-important", &[(2, None)])],
        );
        scss(
            "a {\n\t// stylelint-disable declaration-no-important\n\t// Unrelated\n\tcolor: pink !important;\n}",
            &[("all", &[]), ("declaration-no-important", &[(2, None)])],
        );
        scss(
            "a {\n\t/* stylelint-disable declaration-no-important */\n\n\t// Unrelated\n\tcolor: pink !important;\n}",
            &[("all", &[]), ("declaration-no-important", &[(2, None)])],
        );
        scss(
            "a {\n\t// Unrelated\n\t// stylelint-disable declaration-no-important\n\tcolor: pink !important;\n}",
            &[("all", &[]), ("declaration-no-important", &[(3, None)])],
        );
        scss(
            "a {\n\t// stylelint-disable declaration-no-important\n\t// stylelint-disable foo-bar\n\tcolor: pink !important;\n}",
            &[
                ("all", &[]),
                ("declaration-no-important", &[(2, None)]),
                ("foo-bar", &[(3, None)]),
            ],
        );
        scss(
            "a {\n\t// stylelint-disable declaration-no-important --\n\t// Description 1\n\t// stylelint-disable foo-bar\n\t// --\n\t// Description 2\n\tcolor: pink !important;\n}",
            &[
                ("all", &[]),
                ("declaration-no-important", &[(2, None)]),
                ("foo-bar", &[(4, None)]),
            ],
        );
        scss(
            "a {\n\t// stylelint-disable declaration-no-important\n\t// Unrelated\n\t// stylelint-disable foo-bar\n\tcolor: pink !important;\n}",
            &[
                ("all", &[]),
                ("declaration-no-important", &[(2, None)]),
                ("foo-bar", &[(4, None)]),
            ],
        );
    }

    #[test]
    fn stx_prefix_and_all_keyword() {
        check(
            "/* stx-disable-line foo */",
            &[("all", &[]), ("foo", &[(1, Some(1))])],
        );
        check(
            "/* stylelint-disable all */\na {}",
            &[("all", &[(1, None)])],
        );
        check(
            "/* stylelint-disabled */\n/* stylelint-disable-lines foo */",
            &[("all", &[])],
        );
    }

    #[test]
    fn suppression_lookup() {
        let d = parse(
            "/* stylelint-disable */\na {}\n/* stylelint-enable foo */\nb {}\n/* stylelint-enable */\nc {}",
            Syntax::Css,
        );
        assert!(d.is_suppressed("bar", 2) && d.is_suppressed("bar", 5));
        assert!(!d.is_suppressed("bar", 6));
        assert!(d.is_suppressed("foo", 2) && d.is_suppressed("foo", 3));
        assert!(!d.is_suppressed("foo", 4));

        let d = parse("a {} /* stylelint-disable-line foo */\nb {}", Syntax::Css);
        assert!(d.is_suppressed("foo", 1));
        assert!(!d.is_suppressed("foo", 2) && !d.is_suppressed("bar", 1));
    }
}

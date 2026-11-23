use crate::line_index::LineIndex;
use oxc_css_parser::token::Comment;

#[derive(Debug, Clone)]
struct SuppressionRange {
    rule: Option<String>,
    start_line: usize,
    end_line: usize,
}

#[derive(Debug, Default)]
pub struct DisableDirectives {
    ranges: Vec<SuppressionRange>,
}

impl DisableDirectives {
    pub fn parse(comments: &[Comment<'_>], source: &str, line_index: &LineIndex) -> Self {
        let mut ranges: Vec<SuppressionRange> = Vec::new();
        for c in comments {
            let start = c.span.start as usize;
            let end = c.span.end as usize;
            if end < start + 4 || end > source.len() {
                continue;
            }
            let raw = &source[start + 2..end - 2];
            let trimmed = raw.trim();
            let content = trimmed.split("--").next().unwrap_or(trimmed).trim();
            let Some((directive, rules_str)) = Self::strip_directive(content) else {
                continue;
            };
            let (comment_line, _) = line_index.line_col(source, c.span.start);
            let (comment_end_line, _) = line_index.line_col(source, c.span.end.saturating_sub(1));
            match directive {
                "disable-line" => {
                    for rule in Self::parse_rule_list(rules_str) {
                        ranges.push(SuppressionRange {
                            rule: rule.map(str::to_owned),
                            start_line: comment_line,
                            end_line: comment_line,
                        });
                    }
                }
                "disable-next-line" => {
                    let target_line = comment_end_line + 1;
                    for rule in Self::parse_rule_list(rules_str) {
                        ranges.push(SuppressionRange {
                            rule: rule.map(str::to_owned),
                            start_line: target_line,
                            end_line: target_line,
                        });
                    }
                }
                "disable" => {
                    for rule in Self::parse_rule_list(rules_str) {
                        ranges.push(SuppressionRange {
                            rule: rule.map(str::to_owned),
                            start_line: comment_line,
                            end_line: usize::MAX,
                        });
                    }
                }
                "enable" => {
                    for rule in Self::parse_rule_list(rules_str) {
                        for r in ranges.iter_mut().rev() {
                            if r.end_line == usize::MAX
                                && (rule.is_none() || r.rule.as_deref() == rule)
                            {
                                r.end_line = comment_line;
                                if rule.is_some() {
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Self { ranges }
    }

    fn strip_directive(s: &str) -> Option<(&str, &str)> {
        let rest = s
            .strip_prefix("stylelint-")
            .or_else(|| s.strip_prefix("stx-"))?;
        for cmd in ["disable-next-line", "disable-line", "disable", "enable"] {
            if let Some(after) = rest.strip_prefix(cmd) {
                if after.is_empty() || after.starts_with(char::is_whitespace) {
                    return Some((cmd, after.trim()));
                }
            }
        }
        None
    }

    fn parse_rule_list(rest: &str) -> impl Iterator<Item = Option<&str>> {
        let list = if rest.is_empty() { "all" } else { rest };
        list.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| (s != "all").then_some(s))
    }

    pub fn is_suppressed(&self, rule_name: &str, line: usize) -> bool {
        self.ranges.iter().any(|r| {
            line >= r.start_line
                && line <= r.end_line
                && r.rule.as_deref().is_none_or(|name| name == rule_name)
        })
    }
}

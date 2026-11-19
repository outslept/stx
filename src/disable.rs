use crate::line_index::LineIndex;
use oxc_css_parser::token::Comment;

#[derive(Debug)]
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

        for comment in comments {
            let trimmed = comment.content.trim();
            let content = trimmed
                .split_once("--")
                .map(|(before, _)| before.trim())
                .unwrap_or(trimmed);

            let (start_line, _) = line_index.line_col(source, comment.span.start);
            let (end_line, _) = line_index.line_col(source, comment.span.end);

            if let Some(rest) = Self::strip_directive(content, "disable-next-line") {
                for rule in Self::parse_rule_list(rest) {
                    ranges.push(SuppressionRange {
                        rule,
                        start_line: end_line + 1,
                        end_line: end_line + 1,
                    });
                }
            } else if let Some(rest) = Self::strip_directive(content, "disable-line") {
                for rule in Self::parse_rule_list(rest) {
                    ranges.push(SuppressionRange {
                        rule,
                        start_line,
                        end_line: start_line,
                    });
                }
            } else if let Some(rest) = Self::strip_directive(content, "enable") {
                for rule in Self::parse_rule_list(rest) {
                    for range in ranges.iter_mut().rev() {
                        if range.end_line == usize::MAX && (rule.is_none() || range.rule == rule) {
                            range.end_line = start_line;
                        }
                    }
                }
            } else if let Some(rest) = Self::strip_directive(content, "disable") {
                for rule in Self::parse_rule_list(rest) {
                    ranges.push(SuppressionRange {
                        rule,
                        start_line,
                        end_line: usize::MAX,
                    });
                }
            }
        }

        Self { ranges }
    }

    fn strip_directive<'a>(content: &'a str, suffix: &str) -> Option<&'a str> {
        for prefix in ["stylelint-", "stx-"] {
            if let Some(after_prefix) = content.strip_prefix(prefix) {
                if let Some(rest) = after_prefix.strip_prefix(suffix) {
                    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                        return Some(rest.trim());
                    }
                }
            }
        }
        None
    }

    fn parse_rule_list(rest: &str) -> Vec<Option<String>> {
        if rest.is_empty() || rest == "all" {
            return vec![None];
        }
        rest.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s == "all" {
                    None
                } else {
                    Some(s.to_string())
                }
            })
            .collect()
    }

    pub fn is_suppressed(&self, rule_name: &str, line: usize) -> bool {
        self.ranges.iter().any(|r| {
            line >= r.start_line
                && line <= r.end_line
                && match &r.rule {
                    None => true,
                    Some(target) => target == rule_name,
                }
        })
    }
}

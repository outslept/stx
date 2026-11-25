use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;
use serde_json::{Map, Value};

use crate::context::{Fix, LintContext};
use crate::rules::Rule;

pub const EXPECTED_MSG: &str = "Expected empty line before custom property";
pub const REJECTED_MSG: &str = "Expected no empty line before custom property";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyLineMode {
    Always,
    Never,
}

#[derive(Debug, Clone, Default)]
struct SecondaryOptions {
    except_after_block: bool,
    except_after_comment: bool,
    except_after_custom_property: bool,
    except_first_nested: bool,
    ignore_after_comment: bool,
    ignore_after_custom_property: bool,
    ignore_first_nested: bool,
    ignore_inside_single_line_block: bool,
}

crate::define_stx_lint! {
    /// Require or forbid a blank line before custom property declarations.
    ///
    /// Set the primary option to `"always"` to require a blank line or `"never"`
    /// to forbid one. This checks declarations within blocks, not `var()` uses.
    ///
    /// - `except` reverses the expectation for `after-block`, `after-comment`,
    ///   `after-custom-property`, or `first-nested`.
    /// - `ignore` skips `after-comment`, `after-custom-property`, `first-nested`,
    ///   or `inside-single-line-block`.
    ///
    /// With `"always"`, keep the blank line before `--accent`:
    /// ```css
    /// :root {
    ///   color: black;
    ///
    ///   --accent: red;
    /// }
    /// ```
    "custom-property-empty-line-before", style, fix = fix;
    #[derive(Debug)]
    pub struct CustomPropertyEmptyLineBefore {
        mode: EmptyLineMode,
        options: SecondaryOptions,
    }
}

impl CustomPropertyEmptyLineBefore {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Option<Self> {
        let mode = match primary.as_str()? {
            "always" => EmptyLineMode::Always,
            "never" => EmptyLineMode::Never,
            _ => return None,
        };

        let mut options = SecondaryOptions::default();
        if let Some(sec) = secondary {
            options.except_after_block = has_option(sec, "except", "after-block");
            options.except_after_comment = has_option(sec, "except", "after-comment");
            options.except_after_custom_property =
                has_option(sec, "except", "after-custom-property");
            options.except_first_nested = has_option(sec, "except", "first-nested");

            options.ignore_after_comment = has_option(sec, "ignore", "after-comment");
            options.ignore_after_custom_property =
                has_option(sec, "ignore", "after-custom-property");
            options.ignore_first_nested = has_option(sec, "ignore", "first-nested");
            options.ignore_inside_single_line_block =
                has_option(sec, "ignore", "inside-single-line-block");
        }

        Some(Self { mode, options })
    }
}

fn has_option(sec: &Map<String, Value>, key: &str, needle: &str) -> bool {
    match sec.get(key) {
        Some(Value::String(s)) => s == needle,
        Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some(needle)),
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildKind {
    Comment,
    Block,
    CustomProperty,
    OtherStmt,
}

#[derive(Debug, Clone, Copy)]
struct ChildItem {
    kind: ChildKind,
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct BlockState {
    stmt_start: usize,
    open_brace_pos: usize,
    close_brace_pos: usize,
    last_token_end: usize,
    items: Vec<ChildItem>,
}

fn detect_newline(source: &str) -> &'static str {
    let bytes = source.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'\n' {
            if i > 0 && bytes[i - 1] == b'\r' {
                return "\r\n";
            }
            return "\n";
        }
    }
    "\n"
}

fn has_newline_between(source: &str, start: usize, end: usize) -> bool {
    if start >= end || end > source.len() {
        return false;
    }
    source.as_bytes()[start..end].contains(&b'\n')
}

fn has_empty_line(raw_before: &str) -> bool {
    raw_before.bytes().filter(|&b| b == b'\n').count() >= 2
}

fn is_first_nested(source: &str, block: &BlockState, decl_start: usize) -> bool {
    for item in &block.items {
        if item.kind != ChildKind::Comment {
            return false;
        }
        // A comment only preserves `first-nested` if it is on the same line as `{`
        // and does not share the line with `decl` on a subsequent line.
        if has_newline_between(source, block.open_brace_pos + 1, item.end) {
            return false;
        }
        if !has_newline_between(source, item.end, decl_start) {
            return false;
        }
    }
    true
}

fn get_previous_non_shared_line_comment_node(
    source: &str,
    block: &BlockState,
    decl_start: usize,
) -> Option<ChildKind> {
    let items = &block.items;
    let n = items.len();
    for idx in (0..n).rev() {
        let item = items[idx];
        if item.kind != ChildKind::Comment {
            return Some(item.kind);
        }

        // Check if this comment shares a line with the following non-comment node (or decl)
        let next_non_comment_start = items[idx + 1..]
            .iter()
            .find(|it| it.kind != ChildKind::Comment)
            .map(|it| it.start)
            .unwrap_or(decl_start);

        if !has_newline_between(source, item.end, next_non_comment_start) {
            continue;
        }

        // Check if this comment shares a line with the preceding non-comment node (or `{`)
        let prev_non_comment_end = items[..idx]
            .iter()
            .rev()
            .find(|it| it.kind != ChildKind::Comment)
            .map(|it| it.end)
            .unwrap_or(block.open_brace_pos + 1);

        if !has_newline_between(source, prev_non_comment_end, item.start) {
            continue;
        }

        return Some(ChildKind::Comment);
    }
    None
}

fn find_matching_close_brace(bytes: &[u8], open_brace_pos: usize) -> usize {
    let len = bytes.len();
    let mut i = open_brace_pos + 1;
    let mut depth = 1usize;
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
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
        i += 1;
    }
    len
}

fn is_custom_property_statement(stmt: &str) -> bool {
    if !stmt.starts_with("--") {
        return false;
    }
    let Some(colon_pos) = stmt.find(':') else {
        return false;
    };
    let prop = stmt[..colon_pos].trim_end();
    if prop.len() <= 2 {
        return false;
    }
    if prop.contains("#{") || prop.contains("@{") || prop.contains("$(") {
        return false;
    }
    prop[2..]
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || !b.is_ascii())
}

impl CustomPropertyEmptyLineBefore {
    fn check_custom_property<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        block: &BlockState,
        decl_start: usize,
        newline: &str,
    ) {
        let source = ctx.source;

        if self.options.ignore_inside_single_line_block
            && !has_newline_between(source, block.open_brace_pos, block.close_brace_pos)
        {
            return;
        }

        let first_nested = is_first_nested(source, block, decl_start);
        let prev_kind = get_previous_non_shared_line_comment_node(source, block, decl_start);
        let after_comment = prev_kind == Some(ChildKind::Comment);
        let after_block = prev_kind == Some(ChildKind::Block);
        let after_custom_property = prev_kind == Some(ChildKind::CustomProperty);

        if self.options.ignore_after_comment && after_comment {
            return;
        }
        if self.options.ignore_after_custom_property && after_custom_property {
            return;
        }
        if self.options.ignore_first_nested && first_nested {
            return;
        }

        let mut expect_empty_line = self.mode == EmptyLineMode::Always;

        if (self.options.except_first_nested && first_nested)
            || (self.options.except_after_block && after_block)
            || (self.options.except_after_comment && after_comment)
            || (self.options.except_after_custom_property && after_custom_property)
        {
            expect_empty_line = !expect_empty_line;
        }

        let raw_before_start = block.last_token_end;
        let raw_before = &source[raw_before_start..decl_start];
        let has_empty_line_before = has_empty_line(raw_before);

        if expect_empty_line == has_empty_line_before {
            return;
        }

        let message = if expect_empty_line {
            EXPECTED_MSG.to_string()
        } else {
            REJECTED_MSG.to_string()
        };

        let fix = if expect_empty_line {
            if let Some(rel_nl) = raw_before.find('\n') {
                let nl_pos = (raw_before_start + rel_nl) as u32;
                Some(Fix::single(
                    Span {
                        start: nl_pos,
                        end: nl_pos + 1,
                    },
                    format!("\n{newline}"),
                ))
            } else if raw_before_start > 0 {
                let prev_pos = raw_before_start - 1;
                let prev_ch = &source[prev_pos..raw_before_start];
                Some(Fix::single(
                    Span {
                        start: prev_pos as u32,
                        end: raw_before_start as u32,
                    },
                    format!("{prev_ch}{newline}{newline}"),
                ))
            } else {
                Some(Fix::single(
                    Span {
                        start: raw_before_start as u32,
                        end: raw_before_start as u32,
                    },
                    format!("{newline}{newline}"),
                ))
            }
        } else {
            let first_nl = raw_before.find('\n');
            let last_nl = raw_before.rfind('\n');
            if let (Some(f), Some(l)) = (first_nl, last_nl) {
                let start = (raw_before_start + f + 1) as u32;
                let end = (raw_before_start + l + 1) as u32;
                Some(Fix::single(Span { start, end }, String::new()))
            } else {
                None
            }
        };

        ctx.report_with_fix(
            self.name(),
            Span {
                start: decl_start as u32,
                end: decl_start as u32,
            },
            message,
            fix,
        );
    }
}

impl Rule for CustomPropertyEmptyLineBefore {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let source = ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let newline = detect_newline(source);

        let mut stack: Vec<BlockState> = Vec::new();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            // Skip whitespace
            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // Closing brace of current block
            if b == b'}' {
                if let Some(popped) = stack.pop() {
                    let block_end = i + 1;
                    if let Some(parent) = stack.last_mut() {
                        parent.items.push(ChildItem {
                            kind: ChildKind::Block,
                            start: popped.stmt_start,
                            end: block_end,
                        });
                        parent.last_token_end = block_end;
                    }
                }
                i += 1;
                continue;
            }

            // Block comment `/* ... */`
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                let comment_start = i;
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                let comment_end = i;
                if let Some(current) = stack.last_mut() {
                    current.items.push(ChildItem {
                        kind: ChildKind::Comment,
                        start: comment_start,
                        end: comment_end,
                    });
                    current.last_token_end = comment_end;
                }
                continue;
            }

            // Statement or block header starting at `stmt_start`
            let stmt_start = i;
            let mut paren_depth = 0usize;
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
                if c == b'(' {
                    paren_depth += 1;
                } else if c == b')' {
                    paren_depth = paren_depth.saturating_sub(1);
                } else if paren_depth == 0 && (c == b'{' || c == b';' || c == b'}') {
                    break;
                }
                i += 1;
            }

            if i < len && bytes[i] == b'{' {
                let open_brace_pos = i;
                let close_brace_pos = find_matching_close_brace(bytes, open_brace_pos);
                stack.push(BlockState {
                    stmt_start,
                    open_brace_pos,
                    close_brace_pos,
                    last_token_end: open_brace_pos + 1,
                    items: Vec::new(),
                });
                i += 1;
            } else {
                let stmt_end = if i < len && bytes[i] == b';' {
                    i + 1
                } else {
                    i
                };
                let stmt_slice = &source[stmt_start..stmt_end];
                let is_custom = is_custom_property_statement(stmt_slice);

                if is_custom && let Some(current) = stack.last() {
                    self.check_custom_property(ctx, current, stmt_start, newline);
                }

                if let Some(current) = stack.last_mut() {
                    current.items.push(ChildItem {
                        kind: if is_custom {
                            ChildKind::CustomProperty
                        } else {
                            ChildKind::OtherStmt
                        },
                        start: stmt_start,
                        end: stmt_end,
                    });
                    current.last_token_end = stmt_end;
                }

                i = stmt_end;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_start};
    use serde_json::json;

    const RULE_NAME: &str = "custom-property-empty-line-before";
    const EXPECTED: &str = "Expected empty line before custom property";
    const REJECTED: &str = "Expected no empty line before custom property";

    #[test]
    fn custom_property_empty_line_before_always() {
        let pass = [
            "a {\n\n --custom-prop: value;\n}",
            "a {\r\n\r\n --custom-prop: value;\r\n}",
            "a{\n\n --custom-prop: value; }",
            "a{\n\n --custom-prop: value;\r\n\r\n --custom-prop2: value;}",
            "a{\n\n --custom-prop: value;\n\r\n --custom-prop2: value;}",
            "a{\n top: 10px;\n\r\n --custom-prop: value;}",
            "a{\n @extends .class;\n\r\n --custom-prop: value;}",
            "a{\n $var: value;\n\r\n --custom-prop: value;}",
        ];
        let fail = [
            reject_start(
                "a {\n--custom-prop: value;\n}",
                "a {\n\n--custom-prop: value;\n}",
                EXPECTED,
                (2, 1),
            )
            .with_edit_info(0, (3, 4, "\n\n")),
            reject_start(
                "a {\r\n --custom-prop: value;\r\n}",
                "a {\r\n\r\n --custom-prop: value;\r\n}",
                EXPECTED,
                (2, 2),
            )
            .with_edit_info(0, (4, 5, "\n\r\n")),
            reject_start(
                "a{\n\n --custom-prop: value; \n --custom-prop2: value;}",
                "a{\n\n --custom-prop: value; \n\n --custom-prop2: value;}",
                EXPECTED,
                (4, 2),
            )
            .with_edit_info(0, (27, 28, "\n\n")),
            reject_start(
                "a{\r\n\r\n --custom-prop: value;\r\n --custom-prop2: value;}",
                "a{\r\n\r\n --custom-prop: value;\r\n\r\n --custom-prop2: value;}",
                EXPECTED,
                (4, 2),
            )
            .with_edit_info(0, (29, 30, "\n\r\n")),
            reject_start(
                "a{\n top: 10px;\n --custom-prop: value;}",
                "a{\n top: 10px;\n\n --custom-prop: value;}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (14, 15, "\n\n")),
            reject_start(
                "a{\r\n @extends .class;\r\n --custom-prop: value;}",
                "a{\r\n @extends .class;\r\n\r\n --custom-prop: value;}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (22, 23, "\n\r\n")),
            reject_start(
                "a{\n $var: value;\n --custom-prop: value;}",
                "a{\n $var: value;\n\n --custom-prop: value;}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (16, 17, "\n\n")),
        ];

        Tester::new(RULE_NAME, json!(["always"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_ignore_after_comment() {
        let pass = [
            "a {\n/* comment */\n--custom-prop: value;\n}",
            "a {\r\n/* comment */\r\nn--custom-prop: value;\r\n}",
        ];
        let fail = [
            reject_start(
                "a {\n --custom-prop: value;\n}",
                "a {\n\n --custom-prop: value;\n}",
                EXPECTED,
                (2, 2),
            )
            .with_edit_info(0, (3, 4, "\n\n")),
            reject_start(
                "a {\ncolor: pink; /* comment */\n--custom-prop: value;\n}",
                "a {\ncolor: pink; /* comment */\n\n--custom-prop: value;\n}",
                EXPECTED,
                (3, 1),
            )
            .with_edit_info(0, (30, 31, "\n\n")),
            reject_start(
                "a {\n/* comment */ --custom-prop: value;\n}",
                "a {\n/* comment */\n\n --custom-prop: value;\n}",
                EXPECTED,
                (2, 15),
            )
            .with_edit_info(0, (16, 17, "/\n\n")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "ignore": ["after-comment"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_ignore_after_custom_property() {
        let pass = [
            "a {\n\n --custom-prop: value; --custom-prop2: value;\n}",
            "a {\r\n\r\n --custom-prop: value; --custom-prop2: value;\r\n}",
            "a {\n\n --custom-prop: value;\n --custom-prop2: value;\n}",
            "a {\n\n --custom-prop: value; /* comment */\n --custom-prop2: value;\n}",
            "a {\r\n\r\n --custom-prop: value;\r\n --custom-prop2: value;\r\n}",
            "a {\n\n --custom-prop: value;\n\n --custom-prop2: value;\n}",
            "a {\r\n\r\n --custom-prop: value;\r\n\r\n --custom-prop2: value;\r\n}",
        ];
        let fail = [
            reject_start(
                "a{\n @extends .class;\n --custom-prop: value;\n}",
                "a{\n @extends .class;\n\n --custom-prop: value;\n}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (20, 21, "\n\n")),
            reject_start(
                "a{\r\n @extends .class;\r\n --custom-prop: value;\r\n}",
                "a{\r\n @extends .class;\r\n\r\n --custom-prop: value;\r\n}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (22, 23, "\n\r\n")),
            reject_start(
                "a{\n @include mixin;\n --custom-prop: value;\n}",
                "a{\n @include mixin;\n\n --custom-prop: value;\n}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (19, 20, "\n\n")),
            reject_start(
                "a{\r\n @include mixin;\r\n --custom-prop: value;\r\n}",
                "a{\r\n @include mixin;\r\n\r\n --custom-prop: value;\r\n}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (21, 22, "\n\r\n")),
            reject_start(
                "a {\n --custom-prop: value;\n}",
                "a {\n\n --custom-prop: value;\n}",
                EXPECTED,
                (2, 2),
            )
            .with_edit_info(0, (3, 4, "\n\n")),
            reject_start(
                "a {\r\n --custom-prop: value;\r\n}",
                "a {\r\n\r\n --custom-prop: value;\r\n}",
                EXPECTED,
                (2, 2),
            )
            .with_edit_info(0, (4, 5, "\n\r\n")),
            reject_start(
                "a {\n  top: 15px;\n  --custom-prop: value;\n}",
                "a {\n  top: 15px;\n\n  --custom-prop: value;\n}",
                EXPECTED,
                (3, 3),
            )
            .with_edit_info(0, (16, 17, "\n\n")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "ignore": ["after-custom-property"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_ignore_first_nested() {
        let pass = [
            "a {\n --custom-prop: value;\n\n --custom-prop2: value;\n}",
            "a {\n\n --custom-prop: value;\n\n --custom-prop2: value;\n}",
            "a {\r\n --custom-prop: value;\r\n\r\n --custom-prop2: value;\r\n}",
            "a {\r\n\r\n --custom-prop: value;\r\n\r\n --custom-prop2: value;\r\n}",
        ];
        let fail = [
            reject_start(
                "a {\n --custom-prop: value;\n --custom-prop2: value;\n}",
                "a {\n --custom-prop: value;\n\n --custom-prop2: value;\n}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (26, 27, "\n\n")),
            reject_start(
                "a {\n\n --custom-prop: value;\n --custom-prop2: value;\n}",
                "a {\n\n --custom-prop: value;\n\n --custom-prop2: value;\n}",
                EXPECTED,
                (4, 2),
            )
            .with_edit_info(0, (27, 28, "\n\n")),
            reject_start(
                "a {\r\n --custom-prop: value;\r\n --custom-prop2: value;\r\n}",
                "a {\r\n --custom-prop: value;\r\n\r\n --custom-prop2: value;\r\n}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (28, 29, "\n\r\n")),
            reject_start(
                "a {\r\n\r\n --custom-prop: value;\r\n --custom-prop2: value;\r\n}",
                "a {\r\n\r\n --custom-prop: value;\r\n\r\n --custom-prop2: value;\r\n}",
                EXPECTED,
                (4, 2),
            )
            .with_edit_info(0, (30, 31, "\n\r\n")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "ignore": ["first-nested"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_ignore_inside_single_line_block() {
        let pass = [
            "a { --custom-prop: value; }",
            "a {\n\n --custom-prop: value;\n}",
        ];
        let fail = [reject_start(
            "a {\n --custom-prop: value;\n}",
            "a {\n\n --custom-prop: value;\n}",
            EXPECTED,
            (2, 2),
        )
        .with_edit_info(0, (3, 4, "\n\n"))];

        Tester::new(
            RULE_NAME,
            json!(["always", { "ignore": ["inside-single-line-block"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_except_first_nested() {
        let pass = [
            "a { --custom-prop: value;\n}",
            "a {\n --custom-prop: value;\n}",
            "a {\r\n --custom-prop: value;\r\n}",
            "a { /* comment */\n --custom-prop: value;\n}",
        ];
        let fail = [
            reject_start(
                "a {\n\n --custom-prop: value;\n}",
                "a {\n --custom-prop: value;\n}",
                REJECTED,
                (3, 2),
            )
            .with_edit_info(0, (4, 5, "")),
            reject_start(
                "a {\r\n\r\n --custom-prop: value;\r\n}",
                "a {\r\n --custom-prop: value;\r\n}",
                REJECTED,
                (3, 2),
            )
            .with_edit_info(0, (5, 7, "")),
            reject_start(
                "a { /* comment*/\n\n --custom-prop: value;\n}",
                "a { /* comment*/\n --custom-prop: value;\n}",
                REJECTED,
                (3, 2),
            )
            .with_edit_info(0, (17, 18, "")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "except": ["first-nested"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_except_after_block() {
        let pass = [
            "a {\n\n --foo: red;\n}",
            "a {\n\n a {} \n --foo: red;}",
            "a {\r\n a {} \r\n --foo: red;}",
            "a {\n\n @media all {} \n --foo: red;}",
            "a {\r\n @media all {} \r\n --foo: red;}",
        ];
        let fail = [
            reject_start(
                "a {\n\n --foo: red;\n a {} \n\n --bar: red;}",
                "a {\n\n --foo: red;\n a {} \n --bar: red;}",
                REJECTED,
                (6, 2),
            )
            .with_edit_info(0, (25, 26, "")),
            reject_start(
                "a {\r\n a {} \r\n\r\n --foo: red;}",
                "a {\r\n a {} \r\n --foo: red;}",
                REJECTED,
                (4, 2),
            )
            .with_edit_info(0, (13, 15, "")),
            reject_start(
                "a {\n\n --foo: red;\n @media all {} \n\n --bar: red;}",
                "a {\n\n --foo: red;\n @media all {} \n --bar: red;}",
                REJECTED,
                (6, 2),
            )
            .with_edit_info(0, (34, 35, "")),
            reject_start(
                "a {\r\n @media all {} \r\n\r\n --foo: red;}",
                "a {\r\n @media all {} \r\n --foo: red;}",
                REJECTED,
                (4, 2),
            )
            .with_edit_info(0, (22, 24, "")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "except": ["after-block"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_except_after_comment() {
        let pass = [
            "a {\n\n --custom-prop: value;\n}",
            "a {\n\n /* I am a comment */ \n --custom-prop2: value;}",
            "a {\r\n /* I am a comment */ \r\n --custom-prop2: value;}",
        ];
        let fail = [
            reject_start(
                "a {\n\n --custom-prop: value;\n /* I am a comment */ \n\n --custom-prop2: value;}",
                "a {\n\n --custom-prop: value;\n /* I am a comment */ \n --custom-prop2: value;}",
                REJECTED,
                (6, 2),
            )
            .with_edit_info(0, (51, 52, "")),
            reject_start(
                "a {\r\n /* I am a comment */ \r\n\r\n --custom-prop2: value;}",
                "a {\r\n /* I am a comment */ \r\n --custom-prop2: value;}",
                REJECTED,
                (4, 2),
            )
            .with_edit_info(0, (29, 31, "")),
            reject_start(
                "a {\ncolor: pink; /* I am a comment */\n--custom-prop2: value;}",
                "a {\ncolor: pink; /* I am a comment */\n\n--custom-prop2: value;}",
                EXPECTED,
                (3, 1),
            )
            .with_edit_info(0, (37, 38, "\n\n")),
            reject_start(
                "a {/* I am a comment */ \n --custom-prop2: value;}",
                "a {/* I am a comment */ \n\n --custom-prop2: value;}",
                EXPECTED,
                (2, 2),
            )
            .with_edit_info(0, (24, 25, "\n\n")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "except": ["after-comment"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_except_after_custom_property() {
        let pass = [
            "a {\n\n --custom-prop: value;\n}",
            "a {\n\n --custom-prop:value; \n --custom-prop2: value;}",
            "a {\n\n --custom-prop:value; \r\n --custom-prop2: value;}",
        ];
        let fail = [
            reject_start(
                "a {\n\n --custom-prop:value;\n\n --custom-prop2: value;}",
                "a {\n\n --custom-prop:value;\n --custom-prop2: value;}",
                REJECTED,
                (5, 2),
            )
            .with_edit_info(0, (27, 28, "")),
            reject_start(
                "a {\r\n\r\n --custom-prop: value;\r\n\r\n --custom-prop2: value;}",
                "a {\r\n\r\n --custom-prop: value;\r\n --custom-prop2: value;}",
                REJECTED,
                (5, 2),
            )
            .with_edit_info(0, (31, 33, "")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "except": ["after-custom-property"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_except_after_comment_first_nested() {
        let pass = [
            "a {\n --custom-prop: value;\n}",
            "a {\n\n /* I am a comment */ \n --custom-prop2: value;}",
            "a { /* comment */\n --custom-prop: value;\n}",
        ];

        Tester::new(
            RULE_NAME,
            json!(["always", { "except": ["after-comment", "first-nested"] }]),
            pass,
            [],
        )
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_always_except_multiple() {
        let pass = [
            "a {\n --custom-prop: value; \n --custom-prop2: value; \n /* comment */ \n --custom-prop3: value;\n\n @extends 'x';\n\n --custom-prop4: value; \n & b {\n prop: value;\n } \n\n --custom-prop5: value; \n }",
        ];

        Tester::new(
            RULE_NAME,
            json!([
                "always",
                { "except": ["first-nested", "after-comment", "after-custom-property"] },
            ]),
            pass,
            [],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_never() {
        let pass = [
            "a {\n --custom-prop: value;\n}",
            "a {\r\n --custom-prop: value;\r\n}",
            "a {\n --custom-prop: value; \n --custom-prop2: value; \n\n /* comment */ \n --custom-prop3: value; \n\n @extends 'x'; \n --custom-prop4: value; \n\n & b {\n prop: value; \n } \n --custom-prop5: value;\n }",
            "a{\n top: 10px;\n --custom-prop: value;}",
            "a{\n @extends .class;\n --custom-prop: value;}",
            "a{\n $var: value;\n --custom-prop: value;}",
        ];
        let fail = [
            reject_start(
                "a {\n\n --custom-prop: value;\n}",
                "a {\n --custom-prop: value;\n}",
                REJECTED,
                (3, 2),
            )
            .with_edit_info(0, (4, 5, "")),
            reject_start(
                "a {\r\n\r\n --custom-prop: value;\r\n}",
                "a {\r\n --custom-prop: value;\r\n}",
                REJECTED,
                (3, 2),
            )
            .with_edit_info(0, (5, 7, "")),
            reject_start(
                "a{\n top: 10px;\n\n --custom-prop: value;}",
                "a{\n top: 10px;\n --custom-prop: value;}",
                REJECTED,
                (4, 2),
            )
            .with_edit_info(0, (15, 16, "")),
            reject_start(
                "a{\r\n @extends .class;\r\n\r\n --custom-prop: value;}",
                "a{\r\n @extends .class;\r\n --custom-prop: value;}",
                REJECTED,
                (4, 2),
            )
            .with_edit_info(0, (23, 25, "")),
            reject_start(
                "a{\n $var: value;\n\n --custom-prop: value;}",
                "a{\n $var: value;\n --custom-prop: value;}",
                REJECTED,
                (4, 2),
            )
            .with_edit_info(0, (17, 18, "")),
        ];

        Tester::new(RULE_NAME, json!(["never"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn custom_property_empty_line_before_never_except_first_nested() {
        let pass = [
            "a {\n\n --custom-prop: value;\n}",
            "a {\n\n --custom-prop: value;\n}",
            "a {\r\n\r\n --custom-prop: value;\r\n}",
        ];
        let fail = [
            reject_start(
                "a {\n --custom-prop: value;\n}",
                "a {\n\n --custom-prop: value;\n}",
                EXPECTED,
                (2, 2),
            )
            .with_edit_info(0, (3, 4, "\n\n")),
            reject_start(
                "a {\r\n --custom-prop: value;\r\n}",
                "a {\r\n\r\n --custom-prop: value;\r\n}",
                EXPECTED,
                (2, 2),
            )
            .with_edit_info(0, (4, 5, "\n\r\n")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["never", { "except": ["first-nested"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_never_except_after_block() {
        let pass = [
            "a {\n --foo: red;\n}",
            "a {\n a {} \n\n --foo: red;}",
            "a {\r\n a {} \r\n\r\n --foo: red;}",
            "a {\n @media all {} \n\n --foo: red;}",
            "a {\r\n @media all {} \r\n\r\n --foo: red;}",
        ];
        let fail = [
            reject_start(
                "a {\n --foo: red;\n a {} \n --bar: value;}",
                "a {\n --foo: red;\n a {} \n\n --bar: value;}",
                EXPECTED,
                (4, 2),
            )
            .with_edit_info(0, (23, 24, "\n\n")),
            reject_start(
                "a {\r\n a {} \r\n --foo: red;}",
                "a {\r\n a {} \r\n\r\n --foo: red;}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (12, 13, "\n\r\n")),
            reject_start(
                "a {\n --foo: red;\n @media all {} \n --bar: red;}",
                "a {\n --foo: red;\n @media all {} \n\n --bar: red;}",
                EXPECTED,
                (4, 2),
            )
            .with_edit_info(0, (32, 33, "\n\n")),
            reject_start(
                "a {\r\n @media all {} \r\n --foo: red;}",
                "a {\r\n @media all {} \r\n\r\n --foo: red;}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (21, 22, "\n\r\n")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["never", { "except": ["after-block"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_never_except_after_comment() {
        let pass = [
            "a {\n --custom-prop: value;\n}",
            "a {\n /* I am a comment */ \n\n --custom-prop2: value;}",
            "a {\r\n /* I am a comment */ \r\n\r\n --custom-prop2: value;}",
        ];
        let fail = [
            reject_start(
                "a {\n --custom-prop: value;\n /* I am a comment */ \n --custom-prop2: value;}",
                "a {\n --custom-prop: value;\n /* I am a comment */ \n\n --custom-prop2: value;}",
                EXPECTED,
                (4, 2),
            )
            .with_edit_info(0, (49, 50, "\n\n")),
            reject_start(
                "a {\r\n /* I am a comment */ \r\n --custom-prop2: value;}",
                "a {\r\n /* I am a comment */ \r\n\r\n --custom-prop2: value;}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (28, 29, "\n\r\n")),
            reject_start(
                "a {/* I am a comment */ \n\n --custom-prop2: value;}",
                "a {/* I am a comment */ \n --custom-prop2: value;}",
                REJECTED,
                (3, 2),
            )
            .with_edit_info(0, (25, 26, "")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["never", { "except": ["after-comment"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_never_except_after_custom_property() {
        let pass = [
            "a {\n --custom-prop: value;\n}",
            "a {\n --custom-prop:value; \n\n --custom-prop2: value;}",
            "a {\n --custom-prop:value; /* comment */\n\n --custom-prop2: value;}",
            "a {\n --custom-prop:value; \r\n\r\n --custom-prop2: value;}",
        ];
        let fail = [
            reject_start(
                "a {\n --custom-prop:value;\n--custom-prop2: value;}",
                "a {\n --custom-prop:value;\n\n--custom-prop2: value;}",
                EXPECTED,
                (3, 1),
            )
            .with_edit_info(0, (25, 26, "\n\n")),
            reject_start(
                "a {\n --custom-prop: value;\n --custom-prop2: value;}",
                "a {\n --custom-prop: value;\n\n --custom-prop2: value;}",
                EXPECTED,
                (3, 2),
            )
            .with_edit_info(0, (26, 27, "\n\n")),
        ];

        Tester::new(
            RULE_NAME,
            json!(["never", { "except": ["after-custom-property"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn custom_property_empty_line_before_never_except_multiple() {
        let pass = [
            "a {\n\n --custom-prop: value; \n\n --custom-prop2: value; \n /* comment */ \n\n --custom-prop3: value;\n\n @extends 'x';\n --custom-prop4: value; \n & b {\n prop: value;\n } \n --custom-prop5: value; \n }",
        ];

        Tester::new(
            RULE_NAME,
            json!([
                "never",
                { "except": ["first-nested", "after-comment", "after-custom-property"] },
            ]),
            pass,
            [],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    #[ignore = "CSS-in-JS syntax not supported"]
    fn custom_property_empty_line_before_css_in_js() {
        Tester::new(
            RULE_NAME,
            json!(["always", { "except": ["first-nested"] }]),
            ["css` --foo: 100px; `;"],
            [],
        )
        .with_custom_syntax("naive-css-in-js")
        .test();
    }
}

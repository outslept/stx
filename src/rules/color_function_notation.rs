//! Rule `color-function-notation`: specify modern or legacy notation for
//! applicable color-functions.
//!
//! Reference: <https://stylelint.io/user-guide/rules/color-function-notation>

use crate::context::LintContext;
use crate::fix::Fix;
use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::rules::Rule;
use crate::utils::options::secondary_has_flag;
use crate::utils::reference::{
    LEGACY_NOTATION_COLOR_FUNCTIONS, WITH_ALPHA_ALIAS_COLOR_FUNCTIONS, in_set_ignore_case,
};

/// Primary option mode for [`ColorFunctionNotation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Space-separated channels with optional `/ <alpha>` and canonical `rgb`/`hsl` names.
    Modern,
    /// Comma-separated arguments (`rgb(r, g, b)` / `rgba(r, g, b, a)`).
    Legacy,
}

crate::define_stx_lint! {
    /// Require modern or legacy argument separators in `rgb()` and `hsl()` and their aliases.
    ///
    /// Set the primary option to `"modern"` for spaces between channels and `/`
    /// before alpha, or `"legacy"` for commas. The secondary option
    /// `ignore: ["with-var-inside"]` skips a call with a direct `var()` argument.
    /// Namespaced Sass calls and calls with top-level hex or Sass variable
    /// arguments are skipped. The rule checks notation, not argument validity.
    /// Only `"modern"` autofixes; it also removes the `a` suffix when fixing a call.
    ///
    /// With `"modern"`:
    /// ```css
    /// a { color: rgba(0, 0, 0, 0.5); } /* reported */
    /// a { color: rgb(0 0 0 / 0.5); }   /* accepted */
    /// ```
    "color-function-notation", style, fix = conditional;
    pub struct ColorFunctionNotation {
        mode: Mode,
        ignore_with_var_inside: bool,
    }
}

impl Default for ColorFunctionNotation {
    fn default() -> Self {
        Self {
            mode: Mode::Modern,
            ignore_with_var_inside: false,
        }
    }
}

impl ColorFunctionNotation {
    /// Creates a new `color-function-notation` rule instance from configuration options.
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Option<Self> {
        let mode = match primary.as_str()? {
            "modern" => Mode::Modern,
            "legacy" => Mode::Legacy,
            _ => return None,
        };
        let ignore_with_var_inside = secondary_has_flag(secondary, "ignore", "with-var-inside");
        Some(Self {
            mode,
            ignore_with_var_inside,
        })
    }

    /// Formats the diagnostic message for a color function using the wrong notation.
    #[inline]
    fn expected_message(&self) -> String {
        let mode_str = match self.mode {
            Mode::Modern => "modern",
            Mode::Legacy => "legacy",
        };
        format!("Expected {mode_str} color-function notation")
    }
}

impl Rule for ColorFunctionNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut i: u32 = 0;

        while i < len {
            let b = byte_at(ctx, i);

            // Skip block comments `/* ... */`.
            if b == b'/' && i + 1 < len && byte_at(ctx, i + 1) == b'*' {
                i = skip_block_comment(ctx, i, len);
                continue;
            }

            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/'
                && i + 1 < len
                && byte_at(ctx, i + 1) == b'/'
                && (i == 0 || byte_at(ctx, i - 1) != b':')
            {
                i = skip_line_comment(ctx, i, len);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                i = skip_string(ctx, i, len);
                continue;
            }

            // Look for function calls `<ident>(`.
            if is_ident_start(b) {
                let fn_start = i;
                let mut fn_end = i + 1;
                while fn_end < len && is_ident_char(byte_at(ctx, fn_end)) {
                    fn_end += 1;
                }

                // Ignore namespaced Sass calls such as `color.rgb(...)`.
                let is_namespaced = fn_start > 0 && byte_at(ctx, fn_start - 1) == b'.';

                if !is_namespaced && fn_end < len && byte_at(ctx, fn_end) == b'(' {
                    let fn_name = ctx.slice_range(fn_start, fn_end);

                    if in_set_ignore_case(LEGACY_NOTATION_COLOR_FUNCTIONS, fn_name)
                        && let Some(close_paren) = find_matching_paren(ctx, fn_end, len)
                    {
                        self.check_color_function(ctx, fn_start, fn_end, close_paren);
                    }
                }

                i = fn_end;
                continue;
            }

            i += 1;
        }
    }
}

/// Represents a direct child node inside a color function's argument list,
/// mirroring `postcss-value-parser` node segmentation.
#[derive(Debug)]
enum ArgNode {
    /// A comma `,` divider along with the whitespace immediately before and after it.
    Comma {
        /// Start byte of the `before` whitespace.
        full_start: u32,
        /// End byte of the `after` whitespace.
        full_end: u32,
        /// Slice of whitespace before `,`.
        before_start: u32,
        before_end: u32,
        /// Slice of whitespace after `,`.
        after_start: u32,
        after_end: u32,
    },
    /// A slash `/` divider.
    Slash,
    /// A `var(...)` function call.
    VarFunction,
    /// A word token that disqualifies the function as a standard CSS color function
    /// (starts with `#`, starts with `$`, or contains `.$`).
    NonStandardWord,
    /// Any other value node (`word`, `function`, `string`, `comment`, `space`).
    Other,
}

impl ColorFunctionNotation {
    /// Inspects a single `rgb`/`rgba`/`hsl`/`hsla` function call at `fn_start..(close_paren + 1)`.
    fn check_color_function<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        fn_start: u32,
        fn_end: u32,
        close_paren: u32,
    ) {
        let nodes = parse_arg_nodes(ctx, fn_end + 1, close_paren);

        // `isStandardSyntaxColorFunction(node)`
        if nodes.iter().any(|n| matches!(n, ArgNode::NonStandardWord)) {
            return;
        }

        // `if (ignoreWithVarInside && containsVariable(nodes)) return;`
        if self.ignore_with_var_inside && nodes.iter().any(|n| matches!(n, ArgNode::VarFunction)) {
            return;
        }

        // `const hasEnoughArguments = nodes.length >= 5;`
        if nodes.len() < 5 {
            return;
        }

        let is_legacy = is_likely_legacy(&nodes);
        match self.mode {
            Mode::Modern if !is_legacy => return,
            Mode::Legacy if is_legacy => return,
            _ => {}
        }

        let full_span = Span {
            start: fn_start,
            end: close_paren + 1,
        };

        let fix = if self.mode == Mode::Modern {
            let orig_call = ctx.slice(full_span);
            let fixed_call = build_modern_fixed_call(ctx, fn_start, fn_end, close_paren, &nodes);
            Some(minimal_edit_fix(full_span, orig_call, &fixed_call))
        } else {
            None
        };

        ctx.report_with_fix(self.name(), full_span, self.expected_message(), fix);
    }
}

/// Checks `isLikelyLegacy(nodes)` from Stylelint:
/// returns `false` if any slash `/` is present, otherwise `true` if there are 2 or 3 commas.
fn is_likely_legacy(nodes: &[ArgNode]) -> bool {
    let mut commas: u32 = 0;
    for node in nodes {
        match node {
            ArgNode::Comma { .. } => commas += 1,
            ArgNode::Slash => return false,
            _ => {}
        }
    }
    commas == 2 || commas == 3
}

/// Constructs the autofixed modern color function call string, matching Stylelint's `modernFixer`.
fn build_modern_fixed_call(
    ctx: &LintContext<'_>,
    fn_start: u32,
    fn_end: u32,
    close_paren: u32,
    nodes: &[ArgNode],
) -> String {
    let fn_name = ctx.slice_range(fn_start, fn_end);
    let fixed_fn_name = if in_set_ignore_case(WITH_ALPHA_ALIAS_COLOR_FUNCTIONS, fn_name) {
        ctx.slice_range(fn_start, fn_end - 1)
    } else {
        fn_name
    };

    let mut out = String::new();
    out.push_str(fixed_fn_name);
    out.push('(');

    let mut cursor = fn_end + 1;
    let mut comma_count: u32 = 0;

    for node in nodes {
        if let ArgNode::Comma {
            full_start,
            full_end,
            before_start,
            before_end,
            after_start,
            after_end,
        } = *node
        {
            if full_start > cursor {
                out.push_str(ctx.slice_range(cursor, full_start));
            }
            let before = ctx.slice_range(before_start, before_end);
            let after = ctx.slice_range(after_start, after_end);

            if comma_count < 2 {
                out.push_str(at_least_one_space(after));
                comma_count += 1;
            } else {
                out.push_str(at_least_one_space(before));
                out.push('/');
                out.push_str(at_least_one_space(after));
            }
            cursor = full_end;
        }
    }

    if close_paren > cursor {
        out.push_str(ctx.slice_range(cursor, close_paren));
    }
    out.push(')');
    out
}

/// Returns `ws` if non-empty, or `" "` if empty (matching Stylelint's `atLeastOneSpace`).
#[inline]
fn at_least_one_space(ws: &str) -> &str {
    if ws.is_empty() { " " } else { ws }
}

/// Computes the minimal edit [`Fix`] between `orig` (at `orig_span`) and `fixed`
/// by trimming their longest common prefix and longest common suffix.
fn minimal_edit_fix(orig_span: Span, orig: &str, fixed: &str) -> Fix {
    let ob = orig.as_bytes();
    let fb = fixed.as_bytes();

    let mut prefix_len = 0;
    while prefix_len < ob.len() && prefix_len < fb.len() && ob[prefix_len] == fb[prefix_len] {
        prefix_len += 1;
    }

    let mut suffix_len = 0;
    while suffix_len < (ob.len() - prefix_len)
        && suffix_len < (fb.len() - prefix_len)
        && ob[ob.len() - 1 - suffix_len] == fb[fb.len() - 1 - suffix_len]
    {
        suffix_len += 1;
    }

    let prefix_u32 = u32::try_from(prefix_len).unwrap_or(0);
    let suffix_u32 = u32::try_from(suffix_len).unwrap_or(0);

    Fix::single(
        Span {
            start: orig_span.start + prefix_u32,
            end: orig_span.end - suffix_u32,
        },
        fixed[prefix_len..(fb.len() - suffix_len)].to_string(),
    )
}

/// Parses direct child nodes of a function call in `args_start..args_end`
/// following `postcss-value-parser` tokenization rules.
fn parse_arg_nodes(ctx: &LintContext<'_>, args_start: u32, args_end: u32) -> Vec<ArgNode> {
    let mut nodes = Vec::new();
    let mut pos = args_start;

    while pos < args_end {
        // Check for whitespace and see if it precedes a `,` or `/` divider.
        let ws_start = pos;
        while pos < args_end && byte_at(ctx, pos).is_ascii_whitespace() {
            pos += 1;
        }
        let ws_end = pos;

        if pos < args_end {
            let b = byte_at(ctx, pos);

            // Comma divider `,` absorbs preceding and following whitespace.
            if b == b',' {
                let comma_pos = pos;
                pos += 1;
                let after_start = pos;
                while pos < args_end && byte_at(ctx, pos).is_ascii_whitespace() {
                    pos += 1;
                }
                let after_end = pos;
                nodes.push(ArgNode::Comma {
                    full_start: ws_start,
                    full_end: after_end,
                    before_start: ws_start,
                    before_end: comma_pos,
                    after_start,
                    after_end,
                });
                continue;
            }

            // Slash divider `/` (when not the start of a comment `/*` or `//`).
            if b == b'/' && !(pos + 1 < args_end && matches!(byte_at(ctx, pos + 1), b'*' | b'/')) {
                pos += 1;
                while pos < args_end && byte_at(ctx, pos).is_ascii_whitespace() {
                    pos += 1;
                }
                nodes.push(ArgNode::Slash);
                continue;
            }
        }

        // If we consumed whitespace that was not attached to a `,` or `/` divider,
        // `postcss-value-parser` emits a `space` node.
        if ws_end > ws_start {
            nodes.push(ArgNode::Other);
        }

        if pos >= args_end {
            break;
        }

        let b = byte_at(ctx, pos);

        // Block comment `/* ... */`.
        if b == b'/' && pos + 1 < args_end && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, args_end);
            nodes.push(ArgNode::Other);
            continue;
        }

        // Quoted string literal.
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, args_end);
            nodes.push(ArgNode::Other);
            continue;
        }

        // Parenthesized group `( ... )`.
        if b == b'(' {
            if let Some(close) = find_matching_paren(ctx, pos, args_end) {
                pos = close + 1;
            } else {
                pos = args_end;
            }
            nodes.push(ArgNode::Other);
            continue;
        }

        // Word or function token.
        let word_start = pos;
        while pos < args_end {
            let cur = byte_at(ctx, pos);
            if cur.is_ascii_whitespace() || matches!(cur, b',' | b'(' | b')' | b'"' | b'\'') {
                break;
            }
            if cur == b'/' {
                break;
            }
            pos += 1;
        }
        let word_end = pos;

        if word_end < args_end && byte_at(ctx, word_end) == b'(' {
            let fn_name = ctx.slice_range(word_start, word_end);
            if let Some(close) = find_matching_paren(ctx, word_end, args_end) {
                pos = close + 1;
            } else {
                pos = args_end;
            }
            if fn_name.eq_ignore_ascii_case("var") {
                nodes.push(ArgNode::VarFunction);
            } else {
                nodes.push(ArgNode::Other);
            }
            continue;
        }

        let word = ctx.slice_range(word_start, word_end);
        if word.starts_with('#') || word.starts_with('$') || word.contains(".$") {
            nodes.push(ArgNode::NonStandardWord);
        } else {
            nodes.push(ArgNode::Other);
        }
    }

    nodes
}

/// Finds the matching `)` for an opening `(` at `open_paren_pos`.
fn find_matching_paren(ctx: &LintContext<'_>, open_paren_pos: u32, len: u32) -> Option<u32> {
    let mut depth: u32 = 0;
    let mut pos = open_paren_pos;

    while pos < len {
        let b = byte_at(ctx, pos);

        if b == b'/' && pos + 1 < len && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, len);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, len);
            continue;
        }
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(pos);
            }
        }

        pos += 1;
    }

    None
}

/// Skips a `/* ... */` block comment starting at `pos`.
fn skip_block_comment(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    pos += 2;
    while pos + 1 < len && !(byte_at(ctx, pos) == b'*' && byte_at(ctx, pos + 1) == b'/') {
        pos += 1;
    }
    if pos + 1 < len { pos + 2 } else { len }
}

/// Skips a `// ...` line comment starting at `pos`.
fn skip_line_comment(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    pos += 2;
    while pos < len && byte_at(ctx, pos) != b'\n' {
        pos += 1;
    }
    pos
}

/// Skips a quoted string literal starting at `pos`.
fn skip_string(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    let quote = byte_at(ctx, pos);
    pos += 1;
    while pos < len {
        let cur = byte_at(ctx, pos);
        if cur == b'\\' {
            pos = (pos + 2).min(len);
            continue;
        }
        if cur == quote {
            return pos + 1;
        }
        pos += 1;
    }
    len
}

/// Returns the byte at `pos` in the source string.
#[inline]
fn byte_at(ctx: &LintContext<'_>, pos: u32) -> u8 {
    ctx.slice_range(pos, pos + 1).as_bytes()[0]
}

/// Returns `true` if `b` can start a CSS identifier.
#[inline]
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80
}

/// Returns `true` if `b` can continue a CSS identifier.
#[inline]
fn is_ident_char(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_warnings};

    const NAME: &str = "color-function-notation";
    const MODERN_MESSAGE: &str = "Expected modern color-function notation";
    const LEGACY_MESSAGE: &str = "Expected legacy color-function notation";

    #[test]
    fn modern() {
        let pass = &[
            "a { color: rgb(0 0 0) }",
            "a { color: rgb(12 122 231 / 0.2) }",
            "a { color: rgb(51 170 51 / 20%) }",
            "a { color: hsl(4.71239rad 60% 70%) }",
            "a { color: hsl(.75turn 60% 70%) }",
            "a { color: hsl(270 60% 50% / .15) }",
            "a { color: hsl(270 60% 50% / 15%) }",
            "a { color: HSL(270 60% 70%) }",
            "a { color: RgB(240 0 0 / 50%) }",
            "a { color: hsl(var(--hue) var(--sat) var(--sat) / var(--alpha)) }",
            "a {\n\tcolor: hsl(270 60% 50%\n\t\t/ 15%)\n}",
            "a {\n\tcolor: hsl(\n\t\t270\n\t\t60%\n\t\t50%\n\t\t/ 15%\n\t)\n}",
            "a {\n\tcolor: hsl(\n\t\t270 /* comment */\n\t\t60%\n\t\t50%\n\t\t/* comment */ / 15%\n\t)\n}",
        ];

        let fail = &[
            reject(
                "a { color: rgb(0, 0, 0) }",
                "a { color: rgb(0 0 0) }",
                MODERN_MESSAGE,
                (1, 12, 1, 24),
            )
            .with_edit_info(0, (16, 20, " 0")),
            reject(
                "a { color: rgba(0, 0, 0, 0) }",
                "a { color: rgb(0 0 0 / 0) }",
                MODERN_MESSAGE,
                (1, 12, 1, 28),
            )
            .with_edit_info(0, (14, 24, "(0 0 0 /")),
            reject(
                "a { color: rgba(0, 0, 0, 50%) }",
                "a { color: rgb(0 0 0 / 50%) }",
                MODERN_MESSAGE,
                (1, 12, 1, 30),
            )
            .with_edit_info(0, (14, 24, "(0 0 0 /")),
            reject(
                "a { color: rgba(0 , 0 , 0 , 50%) }",
                "a { color: rgb(0 0 0 / 50%) }",
                MODERN_MESSAGE,
                (1, 12, 1, 33),
            )
            .with_edit_info(0, (14, 27, "(0 0 0 /")),
            reject(
                "a { color: rgba(0,0,0,50%) }",
                "a { color: rgb(0 0 0 / 50%) }",
                MODERN_MESSAGE,
                (1, 12, 1, 27),
            )
            .with_edit_info(0, (14, 22, "(0 0 0 / ")),
            reject(
                "a { color: rgba(var(--r, 20), var(--g), var(--b), 0.6); }",
                "a { color: rgb(var(--r, 20) var(--g) var(--b) / 0.6); }",
                MODERN_MESSAGE,
                (1, 12, 1, 55),
            )
            .with_edit_info(0, (14, 49, "(var(--r, 20) var(--g) var(--b) /")),
            reject(
                "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30),\n\t\t28%, 50%, 0.2\n\t);\n}",
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% / 0.2\n\t);\n}",
                MODERN_MESSAGE,
                (2, 9, 5, 3),
            )
            .with_edit_info(0, (15, 54, "(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /")),
            reject(
                "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30),\n\t\t28%, 50%,\n\t\t0.2\n\t);\n}",
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
                MODERN_MESSAGE,
                (2, 9, 6, 3),
            )
            .with_edit_info(0, (15, 54, "(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /")),
            reject(
                "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30),\n\t\t28%,\n\t\t50%, 0.2\n\t);\n}",
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28%\n\t\t50% / 0.2\n\t);\n}",
                MODERN_MESSAGE,
                (2, 9, 6, 3),
            )
            .with_edit_info(0, (15, 56, "(\n\t\tcalc(var(--hue) + 30)\n\t\t28%\n\t\t50% /")),
            reject(
                "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30) /* comment */,\n\t\t28%, 50% /* comment */, 0.2\n\t);\n}",
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */ / 0.2\n\t);\n}",
                MODERN_MESSAGE,
                (2, 9, 5, 3),
            )
            .with_edit_info(
                0,
                (15, 82, "(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */ /"),
            ),
            reject(
                "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30) /* comment */,\n\t\t28%, 50% /* comment */\n\t\t, 0.2\n\t);\n}",
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */\n\t\t/ 0.2\n\t);\n}",
                MODERN_MESSAGE,
                (2, 9, 6, 3),
            )
            .with_edit_info(
                0,
                (15, 85, "(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */\n\t\t/"),
            ),
            reject_warnings(
                "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\trgba(30, 75, 115, 1),\n\t\thsl(255, 255, 255)\n\t);\n}",
                "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\trgb(30 75 115 / 1),\n\t\thsl(255 255 255)\n\t);\n}",
                vec![
                    (MODERN_MESSAGE.to_string(), (4, 3, 4, 23)),
                    (MODERN_MESSAGE.to_string(), (5, 3, 5, 21)),
                ],
            )
            .with_edit_info(0, (57, 71, "(30 75 115 /"))
            .with_edit_info(1, (85, 91, " 255")),
        ];

        Tester::new(NAME, json!(["modern"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn legacy() {
        let pass = &[
            "a { color: rgb(0, 0, 0) }",
            "a { color: rgba(12, 122, 231, 0.2) }",
            "a { color: rgba(51,170,51,20%) }",
            "a { color: hsl(4.71239rad, 60%, 70%) }",
            "a { color: hsl(.75turn, 60%, 70%) }",
            "a { color: hsla(270, 60%, 50%, .15) }",
            "a { color: hsla(270, 60%, 50%, 15%) }",
            "a { color: HSL(270, 60%, 70%) }",
            "a { color: RgBA(240, 0, 0, 50%) }",
            "a { color: hsla(var(--hue), var(--sat), var(--sat), var(--alpha)) }",
            "a {\n\tcolor: hsla(270, 60%, 50%\n\t\t, 15%)\n}",
            "a {\n\tcolor: hsla(\n\t\t270,\n\t\t60%,\n\t\t50%,\n\t\t15%\n\t)\n}",
            "a {\n\tcolor: hsla(\n\t\t270, /* comment */\n\t\t60%,\n\t\t50%,\n\t\t/* comment */ 15%\n\t)\n}",
        ];

        let fail = &[
            reject(
                "a { color: rgb(0 0 0) }",
                None,
                LEGACY_MESSAGE,
                (1, 12, 1, 22),
            ),
            reject(
                "a { color: rgb(0 0 0 0) }",
                None,
                LEGACY_MESSAGE,
                (1, 12, 1, 24),
            ),
            reject(
                "a { color: rgb(0 0 0 / 50%) }",
                None,
                LEGACY_MESSAGE,
                (1, 12, 1, 28),
            ),
            reject(
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% / 0.2\n\t);\n}",
                None,
                LEGACY_MESSAGE,
                (2, 9, 5, 3),
            ),
            reject_warnings(
                "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\trgb(30 75 115 / 1),\n\t\thsl(255 255 255)\n\t);\n}",
                None,
                vec![
                    (LEGACY_MESSAGE.to_string(), (4, 3, 4, 21)),
                    (LEGACY_MESSAGE.to_string(), (5, 3, 5, 19)),
                ],
            ),
        ];

        Tester::new(NAME, json!(["legacy"]), pass, fail).test();
    }

    #[test]
    fn modern_scss() {
        let pass = &[
            "a { color: rgb(white, .5); }",
            "$a: rgb(0 0 0);",
            "$a: rgb(0 0 0 / 0);",
            "a { color: rgb($a $b $c / $d) }",
            "a { color: rgb($a, $b, $c, $d) }",
            "a { color: rgb($color-var, 50%); }",
            "a { color: rgba(color.mix(#000, #fff, 35%), 0.6); }",
            "a { color: rgba(#fff, 0.85) }",
            "a { color: rgba($a, $b, $c, $d) }",
            "a { color: rgb(color.$var, 0.1) }",
            "a { color: rgba(color.$var, 0.1) }",
        ];

        let fail = &[
            reject(
                "$a: rgb(0, 0, 0);",
                "$a: rgb(0 0 0);",
                MODERN_MESSAGE,
                (1, 5, 1, 17),
            )
            .with_edit_info(0, (9, 13, " 0")),
            reject(
                "$a: rgb(0, 0, 0, 0);",
                "$a: rgb(0 0 0 / 0);",
                MODERN_MESSAGE,
                (1, 5, 1, 20),
            )
            .with_edit_info(0, (9, 16, " 0 0 /")),
        ];

        Tester::new(NAME, json!(["modern"]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn modern_ignore_with_var_inside() {
        let pass = &[
            "a { color: rgba(var(--bar), 0.5); }",
            "a { color: rgba(VaR(--bar), 0.5); }",
        ];
        let fail = &[];

        Tester::new(
            NAME,
            json!(["modern", { "ignore": "with-var-inside" }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn legacy_ignore_with_var_inside() {
        let pass = &[
            "a { color: rgb(var(--foo) / 0.5); }",
            "a { color: rgb(VaR(--foo) / 0.5); }",
        ];
        let fail = &[];

        Tester::new(
            NAME,
            json!(["legacy", { "ignore": "with-var-inside" }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn legacy_scss() {
        let pass = &[
            "$a: rgb(0, 0, 0);",
            "$a: rgba(0, 0, 0, 0);",
            "a { color: rgba($a, $b, $c, $d) }",
            "a { color: rgb($a $b $c / $d) }",
            "a { color: rgb(color.$a color.$b color.$c / color.$d) }",
        ];
        let fail = &[reject(
            "$a: rgb(0 0 0 / 0);",
            None,
            LEGACY_MESSAGE,
            (1, 5, 1, 19),
        )];

        Tester::new(NAME, json!(["legacy"]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }
}

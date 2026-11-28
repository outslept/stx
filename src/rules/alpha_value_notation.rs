//! Enforces either number (`0.5`) or percentage (`50%`) notation for alpha
//! values (`alpha-value-notation`).
//!
//! # Checked Constructs
//!
//! Alpha values are inspected in two contexts:
//!
//! 1. **Dedicated alpha properties** (case-insensitive):
//!    - `opacity`
//!    - `shape-image-threshold`
//!    - `fill-opacity`
//!    - `flood-opacity`
//!    - `stop-opacity`
//!    - `stroke-opacity`
//!
//!    Every standalone numeric or percentage word at the top level of the
//!    declaration value is treated as an alpha value.
//!
//! 2. **Color functions** (case-insensitive):
//!    - `color()`, `hsl()`, `hsla()`, `rgb()`, `rgba()`, `hwb()`, `lab()`,
//!      `lch()`, `oklab()`, `oklch()`
//!
//!    Both CSS Color 3 legacy comma-separated syntax (where the alpha channel
//!    is the 4th argument of a 4-argument call) and CSS Color 4 modern
//!    slash-separated syntax (where the alpha channel is the first standalone
//!    word after the first top-level `/`) are supported, including nested
//!    relative color syntax such as `rgb(from rgb(127 127 127 / 25%) 0 0 0 / 50%)`.
//!
//! Both standard CSS declarations and SCSS `$variable: ...` declarations are
//! checked.

use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        OptionMatcher, ValueToken, ValueTokenKind, decl_value_span, ident_name, ident_raw,
        in_set_ignore_case, is_standard_syntax_value, options_matches, parse_option_matchers,
        sass_variable_value_span, tokenize_value,
    },
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, SassVariableDeclaration},
};
use serde::Deserialize;
use serde_json::{Map, Value};

/// Primary notation mode configured for [`AlphaValueNotation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AlphaMode {
    /// Require unitless numbers in `[0, 1]` (e.g. `0.5`).
    #[default]
    Number,
    /// Require percentages (e.g. `50%`).
    Percentage,
}

impl AlphaMode {
    /// Returns the opposite notation mode (used when `exceptProperties` matches
    /// the current declaration's property name).
    #[inline]
    const fn inverted(self) -> Self {
        match self {
            Self::Number => Self::Percentage,
            Self::Percentage => Self::Number,
        }
    }
}

crate::define_stx_lint! {
    /// Specify percentage or number notation for alpha-values.
    ///
    /// Enforces either unitless number (`0.5`) or percentage (`50%`) notation
    /// for alpha values in dedicated opacity properties (`opacity`,
    /// `shape-image-threshold`, `fill-opacity`, `flood-opacity`, `stop-opacity`,
    /// `stroke-opacity`) and color functions (`color()`, `hsl()`, `hsla()`,
    /// `rgb()`, `rgba()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`).
    "alpha-value-notation", style, fix = fix,
    short_description = "Specify percentage or number notation for alpha-values.";
    #[derive(Debug, Default)]
    pub struct AlphaValueNotation {
        /// Primary notation mode (`"number"` or `"percentage"`).
        mode: AlphaMode,
        /// Compiled matchers from `{ "exceptProperties": [...] }`.
        except_properties: Vec<OptionMatcher>,
    }
}

/// CSS and SVG properties whose values directly represent an alpha/opacity
/// channel (`ALPHA_PROPS` in Stylelint).
const ALPHA_PROPS: &[&str] = &[
    "opacity",
    "shape-image-threshold",
    "fill-opacity",
    "flood-opacity",
    "stop-opacity",
    "stroke-opacity",
];

/// Color functions capable of carrying an alpha channel (`ALPHA_FUNCTION_NAME`
/// in Stylelint).
const ALPHA_FUNCTION_NAMES: &[&str] = &[
    "color", "hsl", "hsla", "rgb", "rgba", "hwb", "lab", "lch", "oklab", "oklch",
];

/// In CSS Color 3 comma-separated syntax (`rgba(r, g, b, a)` / `hsla(h, s, l, a)`),
/// the alpha channel is the 4th argument when and only when the function has
/// exactly 4 arguments.
const LEGACY_COMMA_ARG_COUNT: usize = 4;

/// Formats the diagnostic message matching Stylelint's `messages.expected`.
fn expected(unfixed: &str, fixed: &str) -> String {
    format!("Expected \"{unfixed}\" to be \"{fixed}\"")
}

impl AlphaValueNotation {
    /// Constructs a configured [`AlphaValueNotation`] rule instance from raw
    /// Stylelint JSON options.
    ///
    /// Invalid or unrecognized primary option values fall back to
    /// [`AlphaMode::Number`].
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        let mode = AlphaMode::deserialize(primary).unwrap_or_default();
        let except_properties = parse_option_matchers(secondary, "exceptProperties");
        Self {
            mode,
            except_properties,
        }
    }

    /// Resolves the required [`AlphaMode`] for `prop_name`, inverting the
    /// primary mode if `prop_name` matches `exceptProperties`.
    #[inline]
    fn effective_mode(&self, prop_name: &str) -> AlphaMode {
        if options_matches(&self.except_properties, prop_name) {
            self.mode.inverted()
        } else {
            self.mode
        }
    }

    /// Scans a declaration or SCSS variable value span for alpha tokens and
    /// emits diagnostics with autofixes for any notation mismatches.
    fn check_value(&self, prop_name: &str, val_span: Span, ctx: &mut LintContext<'_>) {
        let val_raw = ctx.slice_range(val_span.start, val_span.end);
        // Fast path matching Stylelint's `DIGIT.test(declarationValue)` guard.
        if !val_raw.bytes().any(|b| b.is_ascii_digit()) {
            return;
        }
        let is_alpha_prop = in_set_ignore_case(ALPHA_PROPS, prop_name);
        if !is_alpha_prop && !contains_alpha_function(val_raw) {
            return;
        }

        let tokens = tokenize_value(ctx.source, val_span);
        for (i, tok) in tokens.iter().enumerate() {
            if is_alpha_prop {
                // In Stylelint: `const inVal = isAlphaProp && DIGIT.test(node.value)`
                // followed by `findAlphaInValue` which accepts standalone `word` nodes.
                if matches!(
                    tok.kind,
                    ValueTokenKind::Number | ValueTokenKind::Percentage
                ) && is_standalone_word(&tokens, i)
                {
                    self.check_alpha_token(tok, prop_name, ctx);
                }
            } else if tok.kind == ValueTokenKind::Function
                && in_set_ignore_case(
                    ALPHA_FUNCTION_NAMES,
                    tok.raw.strip_suffix('(').unwrap_or(tok.raw),
                )
            {
                let close_idx = find_closing_paren(&tokens, i);
                if let Some(alpha_tok) = find_alpha_in_function(&tokens[i + 1..close_idx]) {
                    self.check_alpha_token(alpha_tok, prop_name, ctx);
                }
            }
        }
    }

    /// Validates a candidate alpha token and reports a fixable diagnostic if
    /// its notation contradicts the effective [`AlphaMode`] for `prop_name`.
    fn check_alpha_token<'a>(
        &self,
        token: &ValueToken<'a>,
        prop_name: &str,
        ctx: &mut LintContext<'a>,
    ) {
        if !matches!(
            token.kind,
            ValueTokenKind::Number | ValueTokenKind::Percentage
        ) || !is_standard_syntax_value(token.raw)
        {
            return;
        }

        let fixed = match (self.effective_mode(prop_name), &token.kind) {
            (AlphaMode::Number, ValueTokenKind::Percentage) => to_number(token.raw),
            (AlphaMode::Percentage, ValueTokenKind::Number) => to_percentage(token.raw),
            _ => None,
        };

        if let Some(fixed) = fixed {
            let msg = expected(token.raw, &fixed);
            ctx.report_with_fix(Self::NAME, token.span, msg, Fix::single(token.span, fixed));
        }
    }
}

impl Rule for AlphaValueNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        // Fall back to the raw property slice when `decl.name` uses SCSS/Less
        // interpolation (e.g. `#{$prop}: rgb(...)`) so color functions in the
        // declaration value are still checked.
        let prop_name = ident_name(&decl.name).unwrap_or_else(|| ident_raw(&decl.name, ctx.source));
        let Some(val_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        self.check_value(prop_name, val_span, ctx);
    }

    fn check_sass_variable_declaration<'a>(
        &self,
        decl: &'a SassVariableDeclaration<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        if let Some(span) = sass_variable_value_span(ctx.source, decl) {
            // Include the leading `$` so `var_name` matches `decl.prop` in
            // `postcss-scss` (`$a`).
            let var_name = ctx.slice_range(decl.span.start, decl.name.span.end);
            self.check_value(var_name, span, ctx);
        }
    }
}

/// Zero-allocation pre-tokenization check equivalent to Stylelint's
/// `ALPHA_FUNCTION = /(?:color|hsla?|rgba?|hwb|lab|lch|oklab|oklch)\(/i`.
///
/// Notice that `ALPHA_FUNCTION` has no leading anchor in Stylelint, so a suffix
/// of an identifier before `(` also passes this fast check and is subsequently
/// filtered out by exact function-name matching on `ValueTokenKind::Function`.
fn contains_alpha_function(val_raw: &str) -> bool {
    let bytes = val_raw.as_bytes();
    for (paren_idx, &b) in bytes.iter().enumerate() {
        // Minimum alpha function name length is 3 (`rgb`, `hsl`, `hwb`, `lab`, `lch`).
        if b == b'(' && paren_idx >= 3 {
            let mut start = paren_idx;
            while start > 0 && bytes[start - 1].is_ascii_alphabetic() {
                start -= 1;
            }
            if in_set_ignore_case(ALPHA_FUNCTION_NAMES, &val_raw[start..paren_idx]) {
                return true;
            }
        }
    }
    false
}

/// Returns `true` if `tok` opens a parenthesized group (`fn(` or `(`).
#[inline]
fn is_open_paren(tok: &ValueToken<'_>) -> bool {
    tok.kind == ValueTokenKind::Function || tok.raw == "("
}

/// Finds the index of the `CloseParen` token matching the opening parenthesis
/// at `open_idx`, or `tokens.len()` if the parenthesis is unclosed.
fn find_closing_paren(tokens: &[ValueToken<'_>], open_idx: usize) -> usize {
    let mut depth = 1usize;
    for (i, tok) in tokens.iter().enumerate().skip(open_idx + 1) {
        if is_open_paren(tok) {
            depth += 1;
        } else if tok.kind == ValueTokenKind::CloseParen {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
    }
    tokens.len()
}

/// Returns the token index immediately following the parenthesized group opened
/// at `open_idx`.
#[inline]
fn skip_parenthesized(tokens: &[ValueToken<'_>], open_idx: usize) -> usize {
    let close_idx = find_closing_paren(tokens, open_idx);
    (close_idx + 1).min(tokens.len())
}

/// Returns `true` if `args` contains a `,` token at parenthesis nesting depth 0.
fn has_top_level_comma(args: &[ValueToken<'_>]) -> bool {
    let mut depth = 0usize;
    for tok in args {
        if is_open_paren(tok) {
            depth += 1;
        } else if tok.kind == ValueTokenKind::CloseParen {
            depth = depth.saturating_sub(1);
        } else if tok.kind == ValueTokenKind::Comma && depth == 0 {
            return true;
        }
    }
    false
}

/// Returns `true` if `tok` corresponds to a `div` or `string` node in
/// `postcss-value-parser` (i.e. not a `word` or `function`).
#[inline]
fn is_non_word_delimiter(tok: &ValueToken<'_>) -> bool {
    matches!(
        tok.kind,
        ValueTokenKind::Comma
            | ValueTokenKind::Slash
            | ValueTokenKind::Colon
            | ValueTokenKind::String
    )
}

/// Returns `true` if `tok` always terminates a `postcss-value-parser` `word`
/// node even when adjacent without whitespace.
#[inline]
fn is_word_boundary(tok: &ValueToken<'_>) -> bool {
    is_open_paren(tok) || is_non_word_delimiter(tok) || tok.kind == ValueTokenKind::CloseParen
}

/// Checks whether `tokens[idx]` forms an entire `postcss-value-parser` `word`
/// node by itself.
///
/// `tokenize_value` splits certain malformed or interpolated CSS words into
/// adjacent tokens with contiguous spans — for example:
/// - `1.foo` -> `Number("1")`, `Other(".")`, `Ident("foo")`
/// - `#{$a}` -> `Other("#")`, `Other("{")`, `Ident("$a")`, `Other("}")`
///
/// In `postcss-value-parser`, those sequences are a single `word` node
/// (`"1.foo"` or `"#{$a}"`). Checking that neither neighbor is span-adjacent
/// guarantees we do not mistake the leading `"1"` of `"1.foo"` for a standalone
/// number.
fn is_standalone_word(tokens: &[ValueToken<'_>], idx: usize) -> bool {
    let prev_glued = idx > 0
        && !is_word_boundary(&tokens[idx - 1])
        && tokens[idx - 1].span.end == tokens[idx].span.start;
    let next_glued = idx + 1 < tokens.len()
        && !is_word_boundary(&tokens[idx + 1])
        && tokens[idx].span.end == tokens[idx + 1].span.start;
    !prev_glued && !next_glued
}

/// Consumes one contiguous `postcss-value-parser` `word` starting at `args[*i]`
/// and advances `*i` past all tokens belonging to that word.
///
/// Returns `Some(&args[start])` if and only if the word consisted of a single
/// token; returns `None` for compound glued words such as `1.foo` or `#{$a}`.
fn next_single_word_token<'t, 'a>(
    args: &'t [ValueToken<'a>],
    i: &mut usize,
) -> Option<&'t ValueToken<'a>> {
    let start = *i;
    *i += 1;
    while *i < args.len()
        && !is_word_boundary(&args[*i])
        && args[*i - 1].span.end == args[*i].span.start
    {
        *i += 1;
    }
    (*i == start + 1).then_some(&args[start])
}

/// Finds the first top-level `/` in `args` and returns the slice of tokens
/// following it.
fn skip_past_top_level_slash<'t, 'a>(args: &'t [ValueToken<'a>]) -> Option<&'t [ValueToken<'a>]> {
    let mut i = 0usize;
    while i < args.len() {
        if is_open_paren(&args[i]) {
            i = skip_parenthesized(args, i);
        } else if args[i].kind == ValueTokenKind::Slash {
            return Some(&args[i + 1..]);
        } else {
            i += 1;
        }
    }
    None
}

/// Extracts the alpha argument token from the interior tokens of a color
/// function call (`findAlphaInFunction` in Stylelint).
///
/// - **Comma-separated legacy syntax** (any top-level `,` present): counts
///   top-level `word` and `function` arguments; if the total count is
///   [`LEGACY_COMMA_ARG_COUNT`] (4) and the 4th argument is a single-token word,
///   returns it.
/// - **Modern slash-separated syntax**: locates the first top-level `/` and
///   returns the first top-level `word` after it (skipping any `function` calls
///   like `var(--alpha)` before it, matching `nodesAfterSlash.find(isValueWord)`).
fn find_alpha_in_function<'t, 'a>(args: &'t [ValueToken<'a>]) -> Option<&'t ValueToken<'a>> {
    if has_top_level_comma(args) {
        let mut count = 0usize;
        let mut fourth_tok: Option<&'t ValueToken<'a>> = None;
        let mut i = 0usize;
        while i < args.len() {
            if is_non_word_delimiter(&args[i]) {
                i += 1;
            } else if is_open_paren(&args[i]) {
                count += 1;
                i = skip_parenthesized(args, i);
            } else {
                let single = next_single_word_token(args, &mut i);
                count += 1;
                if count == LEGACY_COMMA_ARG_COUNT {
                    fourth_tok = single;
                }
            }
        }
        return (count == LEGACY_COMMA_ARG_COUNT)
            .then_some(fourth_tok)
            .flatten();
    }

    let after_slash = skip_past_top_level_slash(args)?;
    let mut i = 0usize;
    while i < after_slash.len() {
        if is_non_word_delimiter(&after_slash[i]) {
            i += 1;
        } else if is_open_paren(&after_slash[i]) {
            // In Stylelint: `nodesAfterSlash.find(isValueWord)` skips `function`
            // nodes (such as `var(--alpha)`) and keeps searching for a `word`.
            i = skip_parenthesized(after_slash, i);
        } else {
            return next_single_word_token(after_slash, &mut i);
        }
    }
    None
}

/// Parses a CSS `<number>` string into a finite `f64`, rejecting trailing
/// decimal points (e.g. `"1."`) that Rust's `f64::from_str` accepts but CSS
/// `<number>` syntax forbids.
fn parse_css_number(raw: &str) -> Option<f64> {
    if raw.ends_with('.') {
        return None;
    }
    raw.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Converts a unitless CSS number string (e.g. `"0.5"`) into a percentage
/// string (e.g. `"50%"`), matching Stylelint's `asPercentage`.
fn to_percentage(value: &str) -> Option<String> {
    let number = parse_css_number(value)?;
    let mut formatted = format_float(number * 100.0);
    formatted.push('%');
    Some(formatted)
}

/// Converts a CSS percentage string (e.g. `"50%"`) into a unitless number
/// string (e.g. `"0.5"`), matching Stylelint's `asNumber`.
fn to_number(value: &str) -> Option<String> {
    let num_str = value.strip_suffix('%')?;
    let number = parse_css_number(num_str)?;
    Some(format_float(number / 100.0))
}

/// Formats a floating-point number to match JavaScript's
/// `Number(n.toPrecision(3))` for CSS alpha values without scientific notation
/// or trailing zeros (e.g. `0.14 * 100.0` -> `"14"`, `0.3 / 100.0` -> `"0.003"`).
fn format_float(n: f64) -> String {
    let clean = if n == 0.0 || !n.is_finite() { 0.0 } else { n };
    let mut formatted = format!("{clean:.6}");
    let trimmed_len = formatted.trim_end_matches('0').trim_end_matches('.').len();
    formatted.truncate(trimmed_len);
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_edit, reject_edit_warnings, reject_warnings};
    use serde_json::json;

    #[test]
    fn number_mode() {
        let pass = [
            "a { opacity: 0.1 }",
            "a { opacity: .5 }",
            "a { opacity: 1 }",
            "a { shape-image-threshold: 0.1 }",
            "a { fill-opacity: 0.1 }",
            "a { flood-opacity: 0.1 }",
            "a { stop-opacity: 0.1 }",
            "a { stroke-opacity: 0.1 }",
            "a { color: rgb(0, 0, 0) }",
            "a { color: rgba(0, 0, 0, 0.1) }",
            "a { color: rgb(0, 0, 0, 0.1) }",
            "a { color: rgb(0 0 0 / 0.1) }",
            "a { color: hsl(0, 0%, 0%) }",
            "a { color: hsla(0, 0%, 0%, 0.1) }",
            "a { color: hsl(0, 0%, 0%, 0.1) }",
            "a { color: hsl(0 0% 0% / 0.1) }",
            "a { color: hwb(0 0% 0%) }",
            "a { color: hwb(0 0% 0% / 0.1) }",
            "a { color: lab(0% 0 0) }",
            "a { color: lab(0% 0 0 / 0.1) }",
            "a { color: lch(0% 0 0) }",
            "a { color: lch(0% 0 0 / 0.1) }",
            "a { color: oklab(0% 0 0) }",
            "a { color: oklab(0% 0 0 / 0.1) }",
            "a { color: oklch(0% 0 0) }",
            "a { color: oklch(0% 0 0 / 0.1) }",
            "a { color: color(display-p3 0 0 0 / 0.1) }",
            "a { opacity: var(--alpha) }",
            "a { color: rgba(0, 0, 0, var(--alpha)) }",
            "a { color: hsl(0 0% 0% / var(--alpha)) }",
            "a { color: rgb(from rgb(127 127 127 / 0.25) 0 0 0 / 0.5) }",
        ];

        let fail = [
            reject(
                "a { opacity: 10% }",
                "a { opacity: 0.1 }",
                expected("10%", "0.1"),
                (1, 14, 1, 17),
            ),
            reject(
                "a { shape-image-threshold: 10% }",
                "a { shape-image-threshold: 0.1 }",
                expected("10%", "0.1"),
                (1, 28, 1, 31),
            ),
            reject(
                "a { fill-opacity: 50% }",
                "a { fill-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 19, 1, 22),
            ),
            reject(
                "a { flood-opacity: 50% }",
                "a { flood-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 20, 1, 23),
            ),
            reject(
                "a { stop-opacity: 50% }",
                "a { stop-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 19, 1, 22),
            ),
            reject(
                "a { stroke-opacity: 50% }",
                "a { stroke-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 21, 1, 24),
            ),
            reject(
                "a { color: rgba(0, 0, 0, 10%) }",
                "a { color: rgba(0, 0, 0, 0.1) }",
                expected("10%", "0.1"),
                (1, 26, 1, 29),
            ),
            reject(
                "a { color: rgb(0 0 0 / 10%) }",
                "a { color: rgb(0 0 0 / 0.1) }",
                expected("10%", "0.1"),
                (1, 24, 1, 27),
            ),
            reject(
                "a { color: hsla(0, 0%, 0%, 10%) }",
                "a { color: hsla(0, 0%, 0%, 0.1) }",
                expected("10%", "0.1"),
                (1, 28, 1, 31),
            ),
            reject(
                "a { color: hsl(0 0% 0% / 10%) }",
                "a { color: hsl(0 0% 0% / 0.1) }",
                expected("10%", "0.1"),
                (1, 26, 1, 29),
            ),
            reject(
                "a { color: hwb(0 0% 0% / 10%) }",
                "a { color: hwb(0 0% 0% / 0.1) }",
                expected("10%", "0.1"),
                (1, 26, 1, 29),
            ),
            reject(
                "a { color: lab(0% 0 0 / 10%) }",
                "a { color: lab(0% 0 0 / 0.1) }",
                expected("10%", "0.1"),
                (1, 25, 1, 28),
            ),
            reject(
                "a { color: lch(0% 0 0 / 10%) }",
                "a { color: lch(0% 0 0 / 0.1) }",
                expected("10%", "0.1"),
                (1, 25, 1, 28),
            ),
            reject(
                "a { color: oklab(0% 0 0 / 10%) }",
                "a { color: oklab(0% 0 0 / 0.1) }",
                expected("10%", "0.1"),
                (1, 27, 1, 30),
            ),
            reject(
                "a { color: oklch(0% 0 0 / 10%) }",
                "a { color: oklch(0% 0 0 / 0.1) }",
                expected("10%", "0.1"),
                (1, 27, 1, 30),
            ),
            reject(
                "a { color: color(display-p3 0 0 0 / 10%) }",
                "a { color: color(display-p3 0 0 0 / 0.1) }",
                expected("10%", "0.1"),
                (1, 37, 1, 40),
            ),
            reject(
                "a { opacity: 14% }",
                "a { opacity: 0.14 }",
                expected("14%", "0.14"),
                (1, 14, 1, 17),
            ),
            reject(
                "a { opacity: 0.3% }",
                "a { opacity: 0.003 }",
                expected("0.3%", "0.003"),
                (1, 14, 1, 18),
            ),
            reject(
                "a { opacity: 10.50% }",
                "a { opacity: 0.105 }",
                expected("10.50%", "0.105"),
                (1, 14, 1, 20),
            ),
            reject(
                "a { OPACITY: /*comment*/10% }",
                "a { OPACITY: /*comment*/0.1 }",
                expected("10%", "0.1"),
                (1, 25, 1, 28),
            ),
            reject_warnings(
                "a { background-image: linear-gradient(to right, hwb(0 0% 0% / 10%) 20%, lch(0% 0 0 / 50%) 50%); }",
                "a { background-image: linear-gradient(to right, hwb(0 0% 0% / 0.1) 20%, lch(0% 0 0 / 0.5) 50%); }",
                vec![
                    (expected("10%", "0.1"), (1, 63, 1, 66)),
                    (expected("50%", "0.5"), (1, 86, 1, 89)),
                ],
            ),
            reject_warnings(
                "a { color: rgb(from rgb(127 127 127 / 25%) 0 0 0 / 50%) }",
                "a { color: rgb(from rgb(127 127 127 / 0.25) 0 0 0 / 0.5) }",
                vec![
                    (expected("50%", "0.5"), (1, 52, 1, 55)),
                    (expected("25%", "0.25"), (1, 39, 1, 42)),
                ],
            ),
        ];

        Tester::new(AlphaValueNotation::NAME, json!("number"), pass, fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn percentage_mode() {
        let pass = [
            "a { opacity: 10% }",
            "a { opacity: 50% }",
            "a { opacity: 100% }",
            "a { shape-image-threshold: 10% }",
            "a { fill-opacity: 10% }",
            "a { flood-opacity: 10% }",
            "a { stop-opacity: 10% }",
            "a { stroke-opacity: 10% }",
            "a { color: rgb(0, 0, 0) }",
            "a { color: rgba(0, 0, 0, 10%) }",
            "a { color: rgb(0 0 0 / 10%) }",
            "a { color: hsl(0, 0%, 0%) }",
            "a { color: hsla(0, 0%, 0%, 10%) }",
            "a { color: hsl(0 0% 0% / 10%) }",
            "a { color: hwb(0 0% 0% / 10%) }",
            "a { color: lab(0% 0 0 / 10%) }",
            "a { color: lch(0% 0 0 / 10%) }",
            "a { color: oklab(0% 0 0 / 10%) }",
            "a { color: oklch(0% 0 0 / 10%) }",
            "a { color: color(display-p3 0 0 0 / 10%) }",
            "a { color: rgb(from rgb(127 127 127 / 25%) 0 0 0 / 50%) }",
        ];

        let fail = [
            reject(
                "a { opacity: 0.1 }",
                "a { opacity: 10% }",
                expected("0.1", "10%"),
                (1, 14, 1, 17),
            ),
            reject(
                "a { opacity: .5 }",
                "a { opacity: 50% }",
                expected(".5", "50%"),
                (1, 14, 1, 16),
            ),
            reject(
                "a { opacity: 0.14 }",
                "a { opacity: 14% }",
                expected("0.14", "14%"),
                (1, 14, 1, 18),
            ),
            reject(
                "a { opacity: 0.003 }",
                "a { opacity: 0.3% }",
                expected("0.003", "0.3%"),
                (1, 14, 1, 19),
            ),
            reject(
                "a { opacity: 0.1050 }",
                "a { opacity: 10.5% }",
                expected("0.1050", "10.5%"),
                (1, 14, 1, 20),
            ),
            reject(
                "a { shape-image-threshold: 0.1 }",
                "a { shape-image-threshold: 10% }",
                expected("0.1", "10%"),
                (1, 28, 1, 31),
            ),
            reject(
                "a { fill-opacity: 0.5 }",
                "a { fill-opacity: 50% }",
                expected("0.5", "50%"),
                (1, 19, 1, 22),
            ),
            reject(
                "a { color: rgba(0, 0, 0, 0.1) }",
                "a { color: rgba(0, 0, 0, 10%) }",
                expected("0.1", "10%"),
                (1, 26, 1, 29),
            ),
            reject(
                "a { color: rgb(0 0 0 / 0.1) }",
                "a { color: rgb(0 0 0 / 10%) }",
                expected("0.1", "10%"),
                (1, 24, 1, 27),
            ),
            reject(
                "a { color: color(display-p3 0 0 0 / 0.1) }",
                "a { color: color(display-p3 0 0 0 / 10%) }",
                expected("0.1", "10%"),
                (1, 37, 1, 40),
            ),
            reject_warnings(
                "a { color: rgb(from rgb(127 127 127 / 0.25) 0 0 0 / 0.5) }",
                "a { color: rgb(from rgb(127 127 127 / 25%) 0 0 0 / 50%) }",
                vec![
                    (expected("0.5", "50%"), (1, 53, 1, 56)),
                    (expected("0.25", "25%"), (1, 39, 1, 43)),
                ],
            ),
        ];

        Tester::new(AlphaValueNotation::NAME, json!("percentage"), pass, fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn number_with_except_properties() {
        let pass = [
            "a { opacity: 50% }",
            "a { shape-image-threshold: 0.5 }",
            "a { color: rgb(0 0 0 / 0.5) }",
        ];

        let fail = [
            reject(
                "a { opacity: 0.5 }",
                "a { opacity: 50% }",
                expected("0.5", "50%"),
                (1, 14, 1, 17),
            ),
            reject(
                "a { shape-image-threshold: 50% }",
                "a { shape-image-threshold: 0.5 }",
                expected("50%", "0.5"),
                (1, 28, 1, 31),
            ),
            reject(
                "a { color: rgb(0 0 0 / 50%) }",
                "a { color: rgb(0 0 0 / 0.5) }",
                expected("50%", "0.5"),
                (1, 24, 1, 27),
            ),
        ];

        Tester::new(
            AlphaValueNotation::NAME,
            json!(["number", { "exceptProperties": ["opacity"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();

        Tester::new(
            AlphaValueNotation::NAME,
            json!(["number", { "exceptProperties": ["/opacity/"] }]),
            ["a { opacity: 50% }", "a { fill-opacity: 50% }"],
            [
                reject(
                    "a { opacity: 0.5 }",
                    "a { opacity: 50% }",
                    expected("0.5", "50%"),
                    (1, 14, 1, 17),
                ),
                reject(
                    "a { fill-opacity: 0.5 }",
                    "a { fill-opacity: 50% }",
                    expected("0.5", "50%"),
                    (1, 19, 1, 22),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn percentage_with_except_properties() {
        let pass = [
            "a { opacity: 0.5 }",
            "a { shape-image-threshold: 50% }",
            "a { color: rgb(0 0 0 / 50%) }",
        ];

        let fail = [
            reject(
                "a { opacity: 50% }",
                "a { opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 14, 1, 17),
            ),
            reject(
                "a { shape-image-threshold: 0.5 }",
                "a { shape-image-threshold: 50% }",
                expected("0.5", "50%"),
                (1, 28, 1, 31),
            ),
            reject(
                "a { color: rgb(0 0 0 / 0.5) }",
                "a { color: rgb(0 0 0 / 50%) }",
                expected("0.5", "50%"),
                (1, 24, 1, 27),
            ),
        ];

        Tester::new(
            AlphaValueNotation::NAME,
            json!(["percentage", { "exceptProperties": ["opacity"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn scss_syntax() {
        Tester::new(
            AlphaValueNotation::NAME,
            json!("number"),
            [
                "$a: rgb(0, 0, 0);",
                "$a: rgba(0, 0, 0, 0.1);",
                "$a: rgb(0 0 0 / 0.1);",
                "a { opacity: $a }",
                "a { color: rgba(0, 0, 0, $a) }",
                "a { color: rgb(0 0 0 / #{$a}) }",
            ],
            [reject(
                "$a: rgb(0 0 0 / 50%);",
                "$a: rgb(0 0 0 / 0.5);",
                expected("50%", "0.5"),
                (1, 17, 1, 20),
            )],
        )
        .with_custom_syntax("postcss-scss")
        .with_fix(true)
        .test();

        Tester::new(
            AlphaValueNotation::NAME,
            json!("percentage"),
            [
                "$a: rgb(0, 0, 0);",
                "$a: rgba(0, 0, 0, 10%);",
                "$a: rgb(0 0 0 / 10%);",
                "a { opacity: $a }",
            ],
            [reject(
                "$a: rgb(0 0 0 / 0.5);",
                "$a: rgb(0 0 0 / 50%);",
                expected("0.5", "50%"),
                (1, 17, 1, 20),
            )],
        )
        .with_custom_syntax("postcss-scss")
        .with_fix(true)
        .test();
    }

    #[test]
    fn compute_edit_info() {
        Tester::new(
            AlphaValueNotation::NAME,
            json!("number"),
            &[],
            [
                reject_edit(
                    "a { opacity: 10% }",
                    expected("10%", "0.1"),
                    (1, 14, 1, 17),
                    (13, 16, "0.1"),
                ),
                reject_edit(
                    "a { color: rgba(0, 0, 0, 10%) }",
                    expected("10%", "0.1"),
                    (1, 26, 1, 29),
                    (25, 28, "0.1"),
                ),
                reject_edit(
                    "a { color: rgb(0 0 0 / 10%) }",
                    expected("10%", "0.1"),
                    (1, 24, 1, 27),
                    (23, 26, "0.1"),
                ),
                reject_edit(
                    "a { color: color(display-p3 0 0 0 / 10%) }",
                    expected("10%", "0.1"),
                    (1, 37, 1, 40),
                    (36, 39, "0.1"),
                ),
                reject_edit_warnings(
                    "a { background-image: linear-gradient(to right, hwb(0 0% 0% / 10%) 20%, lch(0% 0 0 / 50%) 50%); }",
                    vec![
                        (expected("10%", "0.1"), (1, 63, 1, 66), Some((62, 65, "0.1"))),
                        (expected("50%", "0.5"), (1, 86, 1, 89), Some((85, 88, "0.5"))),
                    ],
                ),
                reject_edit_warnings(
                    "a { color: rgb(from rgb(127 127 127 / 25%) 0 0 0 / 50%) }",
                    vec![
                        (expected("50%", "0.5"), (1, 52, 1, 55), Some((51, 54, "0.5"))),
                        (expected("25%", "0.25"), (1, 39, 1, 42), Some((38, 41, "0.25"))),
                    ],
                ),
            ],
        )
        .with_compute_edit_info(true)
        .test();

        Tester::new(
            AlphaValueNotation::NAME,
            json!("percentage"),
            &[],
            [
                reject_edit(
                    "a { opacity: 0.1 }",
                    expected("0.1", "10%"),
                    (1, 14, 1, 17),
                    (13, 16, "10%"),
                ),
                reject_edit(
                    "a { color: rgba(0, 0, 0, 0.1) }",
                    expected("0.1", "10%"),
                    (1, 26, 1, 29),
                    (25, 28, "10%"),
                ),
                reject_edit(
                    "a { color: rgb(0 0 0 / 0.1) }",
                    expected("0.1", "10%"),
                    (1, 24, 1, 27),
                    (23, 26, "10%"),
                ),
                reject_edit_warnings(
                    "a { color: rgb(from rgb(127 127 127 / 0.25) 0 0 0 / 0.5) }",
                    vec![
                        (
                            expected("0.5", "50%"),
                            (1, 53, 1, 56),
                            Some((52, 55, "50%")),
                        ),
                        (
                            expected("0.25", "25%"),
                            (1, 39, 1, 43),
                            Some((38, 42, "25%")),
                        ),
                    ],
                ),
            ],
        )
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn edge_cases_and_regressions() {
        Tester::new(
            AlphaValueNotation::NAME,
            json!("percentage"),
            [
                "a { color: rgba(0, 0, 0, 0.5, 1); }",
                "a { opacity: 1.foo; }",
                "a { opacity: 1.; }",
            ],
            [],
        )
        .with_fix(true)
        .test();

        Tester::new(
            AlphaValueNotation::NAME,
            json!("number"),
            [],
            [reject(
                "a { #{$prop}: rgb(0 0 0 / 50%); }",
                "a { #{$prop}: rgb(0 0 0 / 0.5); }",
                expected("50%", "0.5"),
                (1, 27, 1, 30),
            )],
        )
        .with_custom_syntax("postcss-scss")
        .with_fix(true)
        .test();
    }
}

//! Rule: `alpha-value-notation`
//!
//! Specify percentage or number notation for alpha-values.

use oxc_css_parser::ast::{ComponentValue, Declaration};
use serde_json::{Map, Value};

use crate::{
    context::{AstParent, LintContext},
    fix::Fix,
    rules::{
        Rule,
        value_utils::{
            ValueToken, ValueTokenKind, extract_function_name_and_args, format_clean_float,
            tokenize_value,
        },
    },
    utils::{
        LEGACY_NOTATION_COLOR_FUNCTIONS, OptionMatcher, decl_value_span, ident_name,
        in_set_ignore_case, options_matches_ignore_case, parse_option_matchers,
    },
};

/// Target notation mode for [`AlphaValueNotation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlphaNotationMode {
    /// Require alpha values to be written as percentages (e.g. `50%`).
    #[default]
    Percentage,
    /// Require alpha values to be written as unitless numbers (e.g. `0.5`).
    Number,
}

impl AlphaNotationMode {
    /// Returns the opposite notation mode (used when `exceptProperties` matches).
    #[inline]
    fn flipped(self) -> Self {
        match self {
            Self::Percentage => Self::Number,
            Self::Number => Self::Percentage,
        }
    }
}

/// CSS properties whose values represent an alpha/opacity channel directly.
const ALPHA_PROPERTIES: &[&str] = &[
    "opacity",
    "fill-opacity",
    "flood-opacity",
    "stop-opacity",
    "stroke-opacity",
    "shape-image-threshold",
];

/// Color functions that may contain an alpha channel argument.
const ALPHA_COLOR_FUNCTIONS: &[&str] = &[
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color",
];

/// ## `alpha-value-notation`
///
/// Specify percentage or number notation for alpha-values (`"percentage"` or `"number"`).
///
/// ### What it does
///
/// Enforces consistent notation (`0.5` vs `50%`) for alpha values in:
/// - Alpha-specific properties (`opacity`, `fill-opacity`, `flood-opacity`, `stop-opacity`,
///   `stroke-opacity`, `shape-image-threshold`)
/// - Color functions (`rgb()`, `rgba()`, `hsl()`, `hsla()`, `hwb()`, `lab()`, `lch()`,
///   `oklab()`, `oklch()`, `color()`)
///
/// ### Why is this bad?
///
/// Mixing number (`0.5`) and percentage (`50%`) notations for alpha channels across a
/// stylesheet hurts readability and consistency.
///
/// ### Options
///
/// #### Primary
///
/// - `"percentage"`: Alpha values must always use percentage notation (e.g. `50%`).
/// - `"number"`: Alpha values must always use number notation (e.g. `0.5`).
///
/// ```json
/// {
///   "alpha-value-notation": "percentage"
/// }
/// ```
///
/// #### Secondary (optional)
///
/// - `"exceptProperties": ["array", "of", "properties", "/or/", "/regex/"]`: Reverse the
///   primary option for matching property names.
///
/// ```json
/// {
///   "alpha-value-notation": [
///     "percentage",
///     {
///       "exceptProperties": ["opacity"]
///     }
///   ]
/// }
/// ```
///
/// ### Examples
///
/// #### `"percentage"`
///
/// **Incorrect:**
/// ```css
/// a {
///   opacity: 0.5;
///   color: rgb(0 0 0 / 0.5);
/// }
/// ```
///
/// **Correct:**
/// ```css
/// a {
///   opacity: 50%;
///   color: rgb(0 0 0 / 50%);
/// }
/// ```
///
/// #### `"number"`
///
/// **Incorrect:**
/// ```css
/// a {
///   opacity: 50%;
///   color: rgb(0 0 0 / 50%);
/// }
/// ```
///
/// **Correct:**
/// ```css
/// a {
///   opacity: 0.5;
///   color: rgb(0 0 0 / 0.5);
/// }
/// ```
///
/// ### References
///
/// - [Stylelint Documentation: `alpha-value-notation`](https://stylelint.io/user-guide/rules/alpha-value-notation)
#[derive(Default)]
pub struct AlphaValueNotation {
    /// Primary notation mode (`"percentage"` or `"number"`).
    pub mode: AlphaNotationMode,
    /// Compiled matchers for the `"exceptProperties"` secondary option.
    pub except_properties: Vec<OptionMatcher>,
}

impl AlphaValueNotation {
    /// Constructs [`AlphaValueNotation`] from primary and secondary configuration options.
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        let mode = if primary.as_str() == Some("number") {
            AlphaNotationMode::Number
        } else {
            AlphaNotationMode::Percentage
        };

        Self {
            mode,
            except_properties: parse_option_matchers(secondary, "exceptProperties"),
        }
    }

    /// Returns the effective [`AlphaNotationMode`] for a given declaration property,
    /// flipping the configured mode if the property matches `"exceptProperties"`.
    fn effective_mode_for_prop(&self, prop: Option<&str>) -> AlphaNotationMode {
        if let Some(p) = prop {
            if options_matches_ignore_case(&self.except_properties, p) {
                return self.mode.flipped();
            }
        }
        self.mode
    }

    /// Finds the property name of the nearest enclosing `Declaration` in the AST ancestry.
    fn enclosing_decl_prop<'a>(&self, ctx: &LintContext<'a>) -> Option<&'a str> {
        for parent in ctx.ancestry.iter().rev() {
            if let AstParent::Declaration(decl) = parent {
                return ident_name(&decl.name);
            }
        }
        None
    }

    /// Validates a single alpha value token against `mode` and reports an autofixable diagnostic
    /// if its notation does not match.
    fn check_token(
        &self,
        token: &ValueToken<'_>,
        mode: AlphaNotationMode,
        ctx: &mut LintContext<'_>,
    ) {
        match mode {
            AlphaNotationMode::Percentage => {
                if token.kind == ValueTokenKind::Number {
                    if let Ok(val) = token.raw.parse::<f64>() {
                        if (0.0..=1.0).contains(&val) {
                            let expected = format!("{}%", format_clean_float(val * 100.0));
                            ctx.report_with_fix(
                                self.name(),
                                token.span,
                                format!("Expected \"{}\" to be \"{}\"", token.raw, expected),
                                Some(Fix::single(token.span, expected)),
                            );
                        }
                    }
                }
            }
            AlphaNotationMode::Number => {
                if token.kind == ValueTokenKind::Percentage {
                    let num_part = &token.raw[..token.raw.len() - 1];
                    if let Ok(val) = num_part.parse::<f64>() {
                        let expected = format_clean_float(val / 100.0);
                        ctx.report_with_fix(
                            self.name(),
                            token.span,
                            format!("Expected \"{}\" to be \"{}\"", token.raw, expected),
                            Some(Fix::single(token.span, expected)),
                        );
                    }
                }
            }
        }
    }
}

impl Rule for AlphaValueNotation {
    fn name(&self) -> &'static str {
        "alpha-value-notation"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };

        if !in_set_ignore_case(ALPHA_PROPERTIES, prop) {
            return;
        }

        let Some(val_span) = decl_value_span(ctx.source, decl) else {
            return;
        };

        let tokens = tokenize_value(ctx.source, val_span);
        if tokens.len() == 1 {
            let mode = self.effective_mode_for_prop(Some(prop));
            self.check_token(&tokens[0], mode, ctx);
        }
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        let ComponentValue::Function(func) = val else {
            return;
        };

        let Some((fn_name, _, args_span)) = extract_function_name_and_args(ctx.source, func.span)
        else {
            return;
        };

        if !in_set_ignore_case(ALPHA_COLOR_FUNCTIONS, fn_name) {
            return;
        }

        let prop = self.enclosing_decl_prop(ctx);
        let mode = self.effective_mode_for_prop(prop);
        let tokens = tokenize_value(ctx.source, args_span);

        let mut depth = 0usize;
        let mut slash_idx = None;
        let mut comma_indices = Vec::new();

        for (idx, tok) in tokens.iter().enumerate() {
            match tok.kind {
                ValueTokenKind::Function => depth += 1,
                ValueTokenKind::CloseParen => depth = depth.saturating_sub(1),
                ValueTokenKind::Slash if depth == 0 => slash_idx = Some(idx),
                ValueTokenKind::Comma if depth == 0 => comma_indices.push(idx),
                _ => {}
            }
        }

        // Modern space-separated syntax: `rgb(0 0 0 / 50%)`
        if let Some(s_idx) = slash_idx {
            let after = &tokens[s_idx + 1..];
            if after.len() == 1 {
                self.check_token(&after[0], mode, ctx);
            }
        } else if comma_indices.len() == 3
            && in_set_ignore_case(LEGACY_NOTATION_COLOR_FUNCTIONS, fn_name)
        {
            // Legacy comma-separated 4-argument syntax: `rgba(0, 0, 0, 50%)`
            let after = &tokens[comma_indices[2] + 1..];
            if after.len() == 1 {
                self.check_token(&after[0], mode, ctx);
            }
        }
    }
}

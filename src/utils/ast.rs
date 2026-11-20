use oxc_css_parser::Span;
use oxc_css_parser::ast::*;

use crate::rules::value_utils::declaration_value_span;
use crate::utils::text::has_interpolation;

/// Returns the span of an `InterpolableIdent`.
pub fn ident_span(ident: &InterpolableIdent<'_>) -> Span {
    match ident {
        InterpolableIdent::Literal(id) => id.span,
        InterpolableIdent::SassInterpolated(id) => id.span,
        InterpolableIdent::LessInterpolated(id) => id.span,
        InterpolableIdent::Placeholder(ph) => ph.span,
    }
}

/// Returns `Some(name)` only if `ident` is a standard literal identifier (no Sass/Less interpolation).
pub fn ident_name<'a>(ident: &'a InterpolableIdent<'a>) -> Option<&'a str> {
    match ident {
        InterpolableIdent::Literal(id) => Some(id.name),
        _ => None,
    }
}

/// Returns the raw source slice of an `InterpolableIdent`.
pub fn ident_raw<'a>(ident: &'a InterpolableIdent<'a>, source: &'a str) -> &'a str {
    match ident {
        InterpolableIdent::Literal(id) => id.raw,
        _ => {
            let sp = ident_span(ident);
            &source[sp.start as usize..(sp.end as usize).min(source.len())]
        }
    }
}

/// Returns `true` if `ident` is a standard literal identifier without interpolation.
pub fn is_standard_syntax_ident(ident: &InterpolableIdent<'_>) -> bool {
    matches!(ident, InterpolableIdent::Literal(_))
}

/// Returns `Some(&Str)` if the string has no Sass/Less interpolation.
pub fn str_literal<'a>(s: &'a InterpolableStr<'a>) -> Option<&'a Str<'a>> {
    match s {
        InterpolableStr::Literal(lit) => Some(lit),
        _ => None,
    }
}

/// Returns `(name, raw, span)` of a standard CSS function call (ignores Sass `module.fn()`).
pub fn function_ident<'a>(func: &'a Function<'a>) -> Option<&'a Ident<'a>> {
    match &func.name {
        FunctionName::Ident(InterpolableIdent::Literal(id)) => Some(id),
        _ => None,
    }
}

/// Returns `true` if a property name is a CSS custom property (`--*`).
pub fn is_custom_property(prop: &str) -> bool {
    prop.starts_with("--")
}

/// Equivalent to Stylelint's `isStandardSyntaxProperty`.
pub fn is_standard_syntax_property(prop: &str) -> bool {
    if prop.is_empty() {
        return false;
    }
    // SCSS var `$var`, Less var `@var`, or interpolation
    if prop.starts_with('$') || prop.starts_with('@') || has_interpolation(prop) {
        return false;
    }
    // Less append property `prop+:` or `prop+_:`
    if prop.ends_with('+') || prop.ends_with("+_") {
        return false;
    }
    true
}

/// Equivalent to Stylelint's `isStandardSyntaxDeclaration`.
pub fn is_standard_syntax_declaration(decl: &Declaration<'_>) -> bool {
    if decl.less_property_merge.is_some() || decl.name_prefix.is_some() {
        return false;
    }
    let Some(name) = ident_name(&decl.name) else {
        return false;
    };
    is_standard_syntax_property(name)
}

/// Equivalent to Stylelint's `isStandardSyntaxValue`.
pub fn is_standard_syntax_value(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('$') || trimmed.starts_with('@') || has_interpolation(trimmed) {
        return false;
    }
    true
}

/// Computes the `Span` of a `Declaration`'s value (between `:` and `!important`/`;`).
pub fn decl_value_span(source: &str, decl: &Declaration<'_>) -> Option<Span> {
    let name_end = ident_span(&decl.name).end;
    let important_start = decl.important.as_ref().map(|imp| imp.span.start);
    declaration_value_span(source, name_end, decl.span.end, important_start)
}

/// Returns `(raw_value_slice, value_span)` for a `Declaration`.
pub fn decl_value_raw<'a>(source: &'a str, decl: &Declaration<'a>) -> Option<(&'a str, Span)> {
    let span = decl_value_span(source, decl)?;
    let slice = &source[span.start as usize..(span.end as usize).min(source.len())];
    Some((slice, span))
}

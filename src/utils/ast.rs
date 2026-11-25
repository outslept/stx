//! Typed `oxc_css_parser` AST and byte-`Span` inspection utilities.

use oxc_css_parser::{
    Span,
    ast::{
        AtRule, Declaration, Function, FunctionName, Ident, InterpolableIdent, InterpolableStr,
        QualifiedRule, Str,
    },
};

use crate::{
    context::AstParent,
    utils::{
        reference::{NESTING_SUPPORTED_AT_KEYWORDS, in_set_ignore_case},
        syntax::{has_interpolation, is_custom_property, is_standard_syntax_property},
    },
};

/// Returns the source [`Span`] of a potentially interpolated identifier.
#[inline]
pub fn ident_span(ident: &InterpolableIdent<'_>) -> Span {
    match ident {
        InterpolableIdent::Literal(lit) => lit.span,
        InterpolableIdent::SassInterpolated(interp) => interp.span,
        InterpolableIdent::LessInterpolated(interp) => interp.span,
        InterpolableIdent::Placeholder(ph) => ph.span,
    }
}

/// Extracts the static string slice of a literal identifier, returning `None`
/// if the identifier contains preprocessor interpolation.
#[inline]
pub fn ident_name<'a>(ident: &'a InterpolableIdent<'a>) -> Option<&'a str> {
    match ident {
        InterpolableIdent::Literal(lit) => Some(lit.name),
        InterpolableIdent::SassInterpolated(_)
        | InterpolableIdent::LessInterpolated(_)
        | InterpolableIdent::Placeholder(_) => None,
    }
}

/// Returns the raw source slice covered by `ident`.
#[inline]
pub fn ident_raw<'a>(ident: &'a InterpolableIdent<'a>, source: &'a str) -> &'a str {
    let sp = ident_span(ident);
    &source[sp.start as usize..sp.end as usize]
}

/// Returns `true` if `ident` is a standard CSS identifier without preprocessor interpolation.
#[inline]
pub fn is_standard_syntax_ident(ident: &InterpolableIdent<'_>) -> bool {
    match ident {
        InterpolableIdent::Literal(lit) => !has_interpolation(lit.name),
        InterpolableIdent::SassInterpolated(_)
        | InterpolableIdent::LessInterpolated(_)
        | InterpolableIdent::Placeholder(_) => false,
    }
}

/// Extracts the underlying [`Str`] node if `s` is a non-interpolated string literal.
#[inline]
pub fn str_literal<'a>(s: &'a InterpolableStr<'a>) -> Option<&'a Str<'a>> {
    match s {
        InterpolableStr::Literal(lit) => Some(lit),
        InterpolableStr::SassInterpolated(_) | InterpolableStr::LessInterpolated(_) => None,
    }
}

/// Extracts the literal [`Ident`] of a CSS function call, returning `None` if the
/// function name is interpolated.
#[inline]
pub fn function_ident<'a>(func: &'a Function<'a>) -> Option<&'a Ident<'a>> {
    match &func.name {
        FunctionName::Ident(InterpolableIdent::Literal(ident)) => Some(ident),
        _ => None,
    }
}

/// Splits a raw function call span (`name(args)`) into `(name_str, name_span, args_span)`.
///
/// Returns `None` if `func_span` is out of bounds or does not contain balanced parentheses.
pub fn extract_function_name_and_args<'a>(
    source: &'a str,
    func_span: Span,
) -> Option<(&'a str, Span, Span)> {
    let raw = source.get(func_span.start as usize..func_span.end as usize)?;
    let open_rel = raw.find('(')?;
    if !raw.ends_with(')') {
        return None;
    }
    let name = &raw[..open_rel];
    let name_span = Span {
        start: func_span.start,
        end: func_span.start + open_rel as u32,
    };
    let args_span = Span {
        start: func_span.start + open_rel as u32 + 1,
        end: func_span.end - 1,
    };
    Some((name, name_span, args_span))
}

/// Returns `true` if `decl` is a standard CSS declaration (excluding SCSS variables
/// and interpolated property names).
pub fn is_standard_syntax_declaration(decl: &Declaration<'_>) -> bool {
    let Some(name) = ident_name(&decl.name) else {
        return false;
    };
    is_standard_syntax_property(name) || is_custom_property(name)
}

/// Returns `true` if `at_rule` is a standard CSS at-rule rather than a preprocessor
/// construct or interpolated directive.
pub fn is_standard_syntax_at_rule(source: &str, at_rule: &AtRule<'_>) -> bool {
    let name = at_rule.name.name;
    if has_interpolation(name) {
        return false;
    }
    if let Some(prelude_sp) = at_rule_prelude_span(source, at_rule) {
        let prelude_raw = &source[prelude_sp.start as usize..prelude_sp.end as usize];
        if has_interpolation(prelude_raw) {
            return false;
        }
    }
    // Less detached rulesets / mixins calling `@foo();` without space after name
    if at_rule.prelude.is_none() && at_rule.block.is_none() {
        let name_end = at_rule.name.span.end as usize;
        let rest = &source[name_end..at_rule.span.end as usize];
        if rest.trim_start().starts_with('(') {
            return false;
        }
    }
    true
}

/// Returns `true` if `rule` is a standard CSS qualified rule rather than a Less
/// mixin definition, guard, or interpolated rule.
pub fn is_standard_syntax_rule(source: &str, rule: &QualifiedRule<'_>) -> bool {
    let prelude_span = rule.selector.span;
    let raw = source
        .get(prelude_span.start as usize..prelude_span.end as usize)
        .unwrap_or("");
    crate::utils::syntax::is_standard_syntax_selector(raw)
}

/// Returns `true` if a declaration with the given `ancestry` is an at-rule descriptor
/// (e.g. inside `@font-face`, `@counter-style`, `@property`, `@page`, `@view-transition`)
/// rather than a standard style property declaration.
pub fn is_descriptor_declaration(ancestry: &[AstParent<'_>]) -> bool {
    for parent in ancestry.iter().rev() {
        if let AstParent::AtRule(at) = parent {
            if !in_set_ignore_case(NESTING_SUPPORTED_AT_KEYWORDS, at.name.name) {
                return true;
            }
        }
    }
    false
}

/// Returns `true` if the current declaration is a `unicode-range` descriptor inside `@font-face`.
pub fn is_unicode_range_descriptor(prop_name: &str, ancestry: &[AstParent<'_>]) -> bool {
    if !prop_name.eq_ignore_ascii_case("unicode-range") {
        return false;
    }
    for parent in ancestry.iter().rev() {
        if let AstParent::AtRule(at) = parent {
            return at.name.name.eq_ignore_ascii_case("font-face");
        }
    }
    false
}

/// Computes the trimmed byte [`Span`] of a declaration's value from raw offsets,
/// excluding the leading `:` and any trailing `!important` annotation or `;`.
pub fn declaration_value_span(
    source: &str,
    name_end: u32,
    decl_end: u32,
    important_start: Option<u32>,
) -> Option<Span> {
    let bytes = source.as_bytes();
    let end_limit = important_start.unwrap_or(decl_end) as usize;
    let mut pos = name_end as usize;
    while pos < end_limit && bytes[pos] != b':' {
        pos += 1;
    }
    if pos >= end_limit || bytes[pos] != b':' {
        return None;
    }
    pos += 1; // skip ':'

    let slice = source.get(pos..end_limit)?;
    let trimmed_start_rel = slice.len() - slice.trim_start().len();
    let trimmed = slice.trim().trim_end_matches(';').trim_end();
    if trimmed.is_empty() {
        return Some(Span {
            start: pos as u32,
            end: pos as u32,
        });
    }
    Some(Span {
        start: (pos + trimmed_start_rel) as u32,
        end: (pos + trimmed_start_rel + trimmed.len()) as u32,
    })
}

/// Returns the byte [`Span`] of `decl`'s value (excluding `!important` and trailing `;`).
pub fn decl_value_span(source: &str, decl: &Declaration<'_>) -> Option<Span> {
    let name_end = ident_span(&decl.name).end;
    let important_start = decl.important.as_ref().map(|imp| imp.span.start);
    declaration_value_span(source, name_end, decl.span.end, important_start)
}

/// Returns both the raw slice and [`Span`] of `decl`'s value (excluding `!important`).
pub fn decl_value_raw<'a>(source: &'a str, decl: &Declaration<'a>) -> Option<(&'a str, Span)> {
    let span = decl_value_span(source, decl)?;
    let raw = source.get(span.start as usize..span.end as usize)?;
    Some((raw, span))
}

/// Computes the byte [`Span`] of an at-rule's prelude (the text between `@name`
/// and `{` or `;`), returning `None` if the prelude is empty.
pub fn at_rule_prelude_span(source: &str, at_rule: &AtRule<'_>) -> Option<Span> {
    let name_end = at_rule.name.span.end as usize;
    let end_limit = if let Some(block) = &at_rule.block {
        block.span.start as usize
    } else {
        let raw = source.get(name_end..at_rule.span.end as usize)?;
        if raw.ends_with(';') {
            at_rule.span.end as usize - 1
        } else {
            at_rule.span.end as usize
        }
    };
    if name_end >= end_limit {
        return None;
    }
    let slice = source.get(name_end..end_limit)?;
    let trimmed = slice.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lead = slice.len() - slice.trim_start().len();
    let start = (name_end + lead) as u32;
    let end = start + trimmed.len() as u32;
    Some(Span { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Syntax::{Css, Less, Sass, Scss};
    use oxc_css_parser::{
        Allocator, ParserBuilder, Syntax,
        ast::{ComponentValue, Statement, Stylesheet},
    };

    #[derive(Default)]
    struct Nodes<'a> {
        decls: Vec<(&'a Declaration<'a>, Vec<AstParent<'a>>)>,
        at_rules: Vec<&'a AtRule<'a>>,
        first_rule: Option<Option<&'a QualifiedRule<'a>>>,
    }

    fn walk<'a>(stmts: &'a [Statement<'a>], anc: &mut Vec<AstParent<'a>>, out: &mut Nodes<'a>) {
        for stmt in stmts {
            if out.first_rule.is_none() {
                out.first_rule = Some(match stmt {
                    Statement::QualifiedRule(r) => Some(r),
                    _ => None,
                });
            }
            match stmt {
                Statement::Declaration(d) => out.decls.push((d, anc.clone())),
                Statement::QualifiedRule(r) => {
                    anc.push(AstParent::QualifiedRule(r));
                    walk(&r.block.statements, anc, out);
                    anc.pop();
                }
                Statement::AtRule(a) => {
                    out.at_rules.push(a);
                    anc.push(AstParent::AtRule(a));
                    if let Some(b) = &a.block {
                        walk(&b.statements, anc, out);
                    }
                    anc.pop();
                }
                Statement::KeyframeBlock(k) => {
                    anc.push(AstParent::KeyframeBlock(k));
                    walk(&k.block.statements, anc, out);
                    anc.pop();
                }
                Statement::LessConditionalQualifiedRule(r) => walk(&r.block.statements, anc, out),
                Statement::LessMixinDefinition(m) => walk(&m.block.statements, anc, out),
                Statement::UnknownQualifiedRule(r) => walk(&r.block.statements, anc, out),
                _ => {}
            }
        }
    }

    fn with_nodes<R>(src: &str, syntax: Syntax, f: impl FnOnce(&Nodes<'_>) -> R) -> R {
        let alloc = Allocator::default();
        let mut parser = ParserBuilder::new(&alloc, src)
            .syntax(syntax)
            .comments()
            .build();
        let sheet = parser
            .parse::<Stylesheet>()
            .unwrap_or_else(|e| panic!("{syntax:?} parse failed for {src:?}: {e:?}"));
        let mut nodes = Nodes::default();
        walk(&sheet.statements, &mut Vec::new(), &mut nodes);
        f(&nodes)
    }

    fn first_decl<R>(src: &str, syntax: Syntax, f: impl FnOnce(&Declaration<'_>) -> R) -> R {
        with_nodes(src, syntax, |n| {
            f(n.decls.first().expect("no declaration").0)
        })
    }

    fn first_at_rule<R>(src: &str, syntax: Syntax, f: impl FnOnce(&AtRule<'_>) -> R) -> R {
        with_nodes(src, syntax, |n| f(n.at_rules.first().expect("no at-rule")))
    }

    fn first_value<R>(src: &str, syntax: Syntax, f: impl FnOnce(&ComponentValue<'_>) -> R) -> R {
        first_decl(src, syntax, |d| f(d.value.first().expect("no value")))
    }

    type ExtractCase<'a> = (&'a str, Span, Option<(&'a str, Span, Span)>);
    type AtRuleCase<'a> = (&'a str, Syntax, &'a [(&'a str, bool)]);

    fn sp(start: u32, end: u32) -> Span {
        Span { start, end }
    }

    #[test]
    fn ident_helpers() {
        let cases: &[(&str, Syntax, Option<&str>, &str, bool)] = &[
            ("a { color: red }", Css, Some("color"), "color", true),
            ("a { --föo: red }", Css, Some("--föo"), "--föo", true),
            ("a { co\\lor: red }", Css, Some("color"), "co\\lor", true),
            ("a { prop#{$var}: 1px }", Scss, None, "prop#{$var}", false),
            ("a { #{$var}: 1px }", Scss, None, "#{$var}", false),
            ("a { @{p}: 1px }", Less, None, "@{p}", false),
            ("a { prop@{p}erty: 1px }", Less, None, "prop@{p}erty", false),
        ];
        for (src, syntax, name, raw, standard) in cases {
            first_decl(src, *syntax, |d| {
                assert_eq!(ident_name(&d.name), *name, "{src:?}");
                assert_eq!(ident_raw(&d.name, src), *raw, "{src:?}");
                let start = src.find(raw).unwrap() as u32;
                assert_eq!(
                    ident_span(&d.name),
                    sp(start, start + raw.len() as u32),
                    "{src:?}"
                );
                assert_eq!(is_standard_syntax_ident(&d.name), *standard, "{src:?}");
            });
        }
    }

    #[test]
    fn str_literal_cases() {
        let cases: &[(&str, Syntax, Option<&str>)] = &[
            ("a { content: 'a\\'b' }", Css, Some("'a\\'b'")),
            ("a { content: \"ü\" }", Css, Some("\"ü\"")),
            ("a { content: '' }", Css, Some("''")),
            ("a { content: \"a#{$b}c\" }", Scss, None),
            ("a { content: \"a@{b}c\" }", Less, None),
            ("a { content: \"plain\" }", Scss, Some("\"plain\"")),
        ];
        for (src, syntax, raw) in cases {
            first_value(src, *syntax, |v| {
                let ComponentValue::InterpolableStr(s) = v else {
                    panic!("not a string: {src:?}")
                };
                let lit = str_literal(s);
                assert_eq!(lit.map(|l| l.raw), *raw, "{src:?}");
                if let Some(l) = lit {
                    assert_eq!(l.span.slice(src), l.raw);
                }
            });
        }
    }

    #[test]
    fn function_ident_cases() {
        let cases: &[(&str, Syntax, Option<&str>)] = &[
            ("a { color: rgb(0 0 0) }", Css, Some("rgb")),
            ("a { transform: translateX(1px) }", Css, Some("translateX")),
            (
                "a { color: -webkit-gradient(a) }",
                Css,
                Some("-webkit-gradient"),
            ),
            ("a { width: darken($c, 10%) }", Scss, Some("darken")),
            ("a { width: math.div(1, 2) }", Scss, None),
            ("a { width: #{$f}(1) }", Scss, None),
        ];
        for (src, syntax, name) in cases {
            first_value(src, *syntax, |v| {
                let ComponentValue::Function(f) = v else {
                    panic!("not a function: {src:?}")
                };
                assert_eq!(function_ident(f).map(|i| i.name), *name, "{src:?}");
            });
        }
    }

    #[test]
    fn extract_function_name_and_args_cases() {
        let cases: &[ExtractCase] = &[
            (
                "rgb(0, 0, 0)",
                sp(0, 12),
                Some(("rgb", sp(0, 3), sp(4, 11))),
            ),
            ("f()", sp(0, 3), Some(("f", sp(0, 1), sp(2, 2)))),
            ("a(b(c))", sp(0, 7), Some(("a", sp(0, 1), sp(2, 6)))),
            ("(x)", sp(0, 3), Some(("", sp(0, 0), sp(1, 2)))),
            ("föo(1)", sp(0, 7), Some(("föo", sp(0, 4), sp(5, 6)))),
            (
                "x: calc(1px + 2px);",
                sp(3, 18),
                Some(("calc", sp(3, 7), sp(8, 17))),
            ),
            ("f(a", sp(0, 3), None),
            ("fa)", sp(0, 3), None),
            ("", sp(0, 0), None),
            ("f()", sp(0, 9), None),
            ("föo(1)", sp(2, 7), None),
        ];
        for (src, span, expected) in cases {
            assert_eq!(
                extract_function_name_and_args(src, *span),
                *expected,
                "{src:?}"
            );
        }
        let src = "a { color: rgb(1 2 3) }";
        first_value(src, Css, |v| {
            let ComponentValue::Function(f) = v else {
                panic!()
            };
            let (name, name_span, args_span) = extract_function_name_and_args(src, f.span).unwrap();
            assert_eq!(
                (name, name_span.slice(src), args_span.slice(src)),
                ("rgb", "rgb", "1 2 3")
            );
        });
    }

    fn decls_standard(src: &str, syntax: Syntax) -> Vec<bool> {
        with_nodes(src, syntax, |n| {
            n.decls
                .iter()
                .map(|(d, _)| is_standard_syntax_declaration(d))
                .collect()
        })
    }

    #[test]
    fn is_standard_syntax_declaration_cases() {
        let cases: &[(&str, Syntax, &[bool])] = &[
            ("a { a: b }", Css, &[true]),
            ("a { a: $b }", Css, &[true]),
            ("a { --custom-property: x }", Css, &[true]),
            ("a { a : calc(b + c) }", Css, &[true]),
            ("@page { size: A4 }", Css, &[true]),
            ("color: yellow;", Css, &[true]),
            ("$var: b", Scss, &[]),
            ("$list: (key: value, key2: value2)", Scss, &[]),
            ("$map: (value, value2)", Scss, &[]),
            ("a { $var: b }", Scss, &[]),
            ("a { $list: (key: value, key2: value2) }", Scss, &[]),
            ("a { $map: (value, value2) }", Scss, &[]),
            ("$foo: (key: value, key2: value2)", Scss, &[]),
            ("$foo: (\"key\": value, \"key2\": value2)", Scss, &[]),
            ("$foo: (value, value2)", Scss, &[]),
            ("namespace.$var: b", Scss, &[]),
            ("a { namespace.$var: b }", Scss, &[]),
            ("$var: b", Sass, &[]),
            ("$list: (key: value, key2: value2)", Sass, &[]),
            ("$map: (value, value2)", Sass, &[]),
            ("a\n\t$var: b", Sass, &[]),
            ("a\n\t$list: (key: value, key2: value2)", Sass, &[]),
            ("a\n\t$map: (value, value2)", Sass, &[]),
            ("a { height: (0) }", Scss, &[true]),
            ("a { height: (3px + 5px) * (2px + 4px) }", Scss, &[true]),
            ("a { &:extend(b) }", Less, &[]),
            ("@map: { key: value; }", Less, &[]),
        ];
        for (src, syntax, expected) in cases {
            assert_eq!(&decls_standard(src, *syntax), expected, "{src:?}");
        }
    }

    #[test]
    #[ignore = "diverges from stylelint: interpolated property names are non-standard in STX"]
    fn is_standard_syntax_declaration_interpolated_props() {
        let cases: &[(&str, Syntax)] = &[
            ("a { #{$var}: 10px; }", Scss),
            ("a { prop#{$var}: 10px; }", Scss),
            ("a { prop#{$var}erty: 10px; }", Scss),
            ("a { @{var}: 10px; }", Less),
            ("a { prop@{var}: 10px; }", Less),
            ("a { prop@{var}erty: 10px; }", Less),
            ("a\n\t#{$var}: 10px", Sass),
            ("a\n\tprop#{$var}: 10px", Sass),
            ("a\n\tprop#{$var}erty: 10px", Sass),
        ];
        let got: Vec<_> = cases
            .iter()
            .map(|(src, syntax)| decls_standard(src, *syntax))
            .collect();
        assert_eq!(got, vec![vec![true]; cases.len()]);
    }

    #[test]
    #[ignore = "diverges from stylelint: declarations inside a Less map `#name()` are standard in STX"]
    fn is_standard_syntax_declaration_less_map() {
        assert_eq!(decls_standard("#my-map() { key: value; }", Less), [false]);
    }

    #[test]
    #[ignore = "diverges from stylelint: a sass nested property parent is a standard declaration in STX"]
    fn is_standard_syntax_declaration_nested_property() {
        let got = [
            decls_standard("a { border: { style: solid; } }", Scss),
            decls_standard("a\n\tborder:\n\t\tstyle: solid", Sass),
        ];
        assert_eq!(got, [[false], [false]]);
    }

    fn at_rules_standard(src: &str, syntax: Syntax) -> Vec<(String, bool)> {
        with_nodes(src, syntax, |n| {
            n.at_rules
                .iter()
                .map(|a| (a.name.name.to_string(), is_standard_syntax_at_rule(src, a)))
                .collect()
        })
    }

    #[test]
    fn is_standard_syntax_at_rule_cases() {
        let cases: &[AtRuleCase] = &[
            ("@import url(foo.css)", Css, &[("import", true)]),
            ("@import 'foo.css';", Css, &[("import", true)]),
            ("@import \"foo.css\";", Css, &[("import", true)]),
            (
                "@import url(\"fineprint.css\") print;",
                Css,
                &[("import", true)],
            ),
            ("@media (min-width: 100px) {};", Css, &[("media", true)]),
            ("@media\n(min-width: 100px) {};", Css, &[("media", true)]),
            ("@media\r\n(min-width: 100px) {};", Css, &[("media", true)]),
            ("@media(min-width: 100px) {};", Css, &[("media", true)]),
            (
                "@detached-ruleset: { background: red; }; .top { @detached-ruleset(); }",
                Less,
                &[],
            ),
            ("a { .mixin(); }", Less, &[]),
            (
                "@my-variable: 10px; .top { margin-top: @my-variable; }",
                Less,
                &[],
            ),
        ];
        for (src, syntax, expected) in cases {
            let got = at_rules_standard(src, *syntax);
            let got: Vec<_> = got.iter().map(|(n, b)| (n.as_str(), *b)).collect();
            assert_eq!(&got, expected, "{src:?}");
        }
    }

    #[test]
    fn is_standard_syntax_at_rule_interpolation() {
        let cases: &[(&str, Syntax, bool)] = &[
            ("@media #{$q} {}", Scss, false),
            ("@media @{q} {}", Less, false),
            ("@include foo;", Scss, true),
            ("@font-face {}", Css, true),
        ];
        for (src, syntax, expected) in cases {
            assert_eq!(
                first_at_rule(src, *syntax, |a| is_standard_syntax_at_rule(src, a)),
                *expected,
                "{src:?}"
            );
        }
    }

    #[test]
    #[ignore = "diverges from stylelint: scss @content is standard in STX"]
    fn is_standard_syntax_at_rule_content() {
        assert_eq!(
            at_rules_standard("@mixin mixin() { @content; };", Scss),
            [("mixin".to_string(), true), ("content".to_string(), false)]
        );
    }

    fn rule_standard(src: &str, syntax: Syntax) -> Option<bool> {
        with_nodes(src, syntax, |n| {
            n.first_rule
                .flatten()
                .map(|r| is_standard_syntax_rule(src, r))
        })
    }

    #[test]
    fn is_standard_syntax_rule_cases() {
        let cases: &[(&str, Syntax, Option<bool>)] = &[
            ("a {}", Css, Some(true)),
            ("when a {}", Css, Some(true)),
            ("a when {}", Css, Some(true)),
            ("a:last-child {}", Css, Some(true)),
            ("a:not(.a) {}", Css, Some(true)),
            ("a::after {}", Css, Some(true)),
            (":--custom-selector {}", Css, Some(true)),
            (":--custom-selector:--custom-selector {}", Css, Some(true)),
            ("--custom-property-set: {}", Css, None),
            ("foo: {};", Css, None),
            (".mixin-name(@var) {}", Less, None),
            (".mixin-name() {}", Less, None),
            (".mixin-name(@a, @b) {}", Less, None),
            (".mixin-name3(@a, @b) {}", Less, None),
            ("#mixin-name() {}", Less, None),
            (".box-shadow(@style, @c) when (iscolor(@c)) {}", Less, None),
            ("@foo: {};", Less, None),
            ("#namespace when (@mode=huge) {}", Less, None),
            (".mixin (@variable: 5) {}", Less, None),
            (".mixin (@variable) when (@variable = 10px) {}", Less, None),
            (".foo() when (@variable = true) {}", Less, None),
            (".foo()when(@variable = true) {}", Less, None),
            (".foo()   when   (@variable = true) {}", Less, None),
            (".foo()\nwhen\n(@variable = true) {}", Less, None),
            (".foo()\r\nwhen\r\n(@variable = true) {}", Less, None),
            (".foo() when (default()) {}", Less, None),
            (".foo() when not (@variable = true) {}", Less, None),
        ];
        for (src, syntax, expected) in cases {
            assert_eq!(rule_standard(src, *syntax), *expected, "{src:?}");
        }
    }

    #[test]
    fn is_standard_syntax_rule_interpolation() {
        let cases: &[(&str, Syntax, Option<bool>)] = &[
            (".a-#{$b} {}", Scss, Some(false)),
            ("%placeholder {}", Scss, Some(false)),
            (".a-@{b} {}", Less, Some(false)),
            ("a, b {}", Scss, Some(true)),
        ];
        for (src, syntax, expected) in cases {
            assert_eq!(rule_standard(src, *syntax), *expected, "{src:?}");
        }
    }

    fn descriptor_flags(src: &str) -> Vec<(bool, bool)> {
        with_nodes(src, Css, |n| {
            n.decls
                .iter()
                .map(|(d, anc)| {
                    let name = ident_name(&d.name).unwrap();
                    (
                        is_descriptor_declaration(anc),
                        is_unicode_range_descriptor(name, anc),
                    )
                })
                .collect()
        })
    }

    #[test]
    fn is_descriptor_declaration_cases() {
        let cases: &[(&str, bool)] = &[
            ("@font-face { font-weight: 400; }", true),
            (
                "@position-try --custom-bottom-right { @supports (position-area: bottom right) { position-area: bottom right; } }",
                true,
            ),
            ("@Counter-STYLE foo { system:\tcyclic; }", true),
            ("a { color: red; }", false),
            ("@media (min-width: 10px) { a { color: red; } }", false),
            ("a { @media (min-width: 10px) { color: red; } }", false),
            ("@property --x { syntax: '*'; }", true),
            ("@page { margin: 0; }", true),
        ];
        for (src, expected) in cases {
            let got: Vec<_> = descriptor_flags(src).into_iter().map(|(d, _)| d).collect();
            assert_eq!(got, [*expected], "{src:?}");
        }
    }

    #[test]
    #[ignore = "diverges from stylelint: nearest rule/keyframe block does not stop the ancestor search"]
    fn is_descriptor_declaration_keyframe_block() {
        let got: Vec<_> = descriptor_flags("@keyframes foo { from { color: red; } }")
            .into_iter()
            .map(|(d, _)| d)
            .collect();
        assert_eq!(got, [false]);
    }

    #[test]
    fn is_unicode_range_descriptor_cases() {
        let cases: &[(&str, bool)] = &[
            ("@font-face { unicode-range: U+0025-00FF; }", true),
            ("@FONT-FACE { UNICODE-RANGE: U+0025; }", true),
            ("@font-face { src: url(a.woff); }", false),
            ("a { unicode-range: U+0025; }", false),
            ("unicode-range: U+0025;", false),
            ("@media print { a { unicode-range: U+0025; } }", false),
            (
                "@font-face { @media print { unicode-range: U+0025; } }",
                false,
            ),
        ];
        for (src, expected) in cases {
            let got: Vec<_> = descriptor_flags(src).into_iter().map(|(_, u)| u).collect();
            assert_eq!(got, [*expected], "{src:?}");
        }
    }

    #[test]
    fn declaration_value_span_cases() {
        let cases: &[(&str, u32, Option<u32>, Option<&str>)] = &[
            ("color: red;", 5, None, Some("red")),
            ("color: red !important", 5, Some(11), Some("red")),
            ("a :  b  ;", 1, None, Some("b")),
            ("a: b;;", 1, None, Some("b")),
            ("a: red /* x */ blue", 1, None, Some("red /* x */ blue")),
            ("a:\r\n  b\r\n", 1, None, Some("b")),
            ("a: 'ü'", 1, None, Some("'ü'")),
            ("a:b", 1, None, Some("b")),
            ("color red", 5, None, None),
            ("a !important: b", 1, Some(2), None),
        ];
        for (src, name_end, imp, expected) in cases {
            let got = declaration_value_span(src, *name_end, src.len() as u32, *imp);
            assert_eq!(got.map(|s| s.slice(src)), *expected, "{src:?}");
        }
        assert_eq!(
            declaration_value_span("color: red;", 5, 11, None),
            Some(sp(7, 10))
        );
        assert_eq!(
            declaration_value_span("color:;", 5, 7, None),
            Some(sp(6, 6))
        );
        assert_eq!(
            declaration_value_span("color:   ", 5, 9, None),
            Some(sp(6, 6))
        );
    }

    #[test]
    fn decl_value_raw_cases() {
        let cases: &[(&str, Syntax, &str)] = &[
            ("a { color: red; }", Css, "red"),
            (
                "a { color: red /* foo */ blue; }",
                Css,
                "red /* foo */ blue",
            ),
            ("a { color: red !important; }", Css, "red"),
            ("a { color: red!IMPORTANT }", Css, "red"),
            ("a { color : \r\n red\r\n}", Css, "red"),
            ("a { --x: { a: b } }", Css, "{ a: b }"),
            ("a { --x:; }", Css, ""),
            ("a { content: 'a;b' }", Css, "'a;b'"),
            (
                "a { width: calc(1px + (2px * 3)) }",
                Css,
                "calc(1px + (2px * 3))",
            ),
            ("a { #{$p}: 1px #{$v} }", Scss, "1px #{$v}"),
            ("a { @{p}: @v }", Less, "@v"),
            ("a { color: red !important }", Scss, "red"),
        ];
        for (src, syntax, expected) in cases {
            first_decl(src, *syntax, |d| {
                let (raw, span) = decl_value_raw(src, d).unwrap();
                assert_eq!(raw, *expected, "{src:?}");
                assert_eq!(decl_value_span(src, d), Some(span), "{src:?}");
                assert_eq!(span.slice(src), raw);
            });
        }
    }

    #[test]
    fn at_rule_prelude_span_cases() {
        let cases: &[(&str, Syntax, Option<&str>)] = &[
            ("@media all {}", Css, Some("all")),
            (
                "@media all /* foo */ and print {}",
                Css,
                Some("all /* foo */ and print"),
            ),
            ("@import url(x.css) print;", Css, Some("url(x.css) print")),
            ("@import 'a.css'", Css, Some("'a.css'")),
            (
                "@media\r\n(min-width: 1px)\r\n{}",
                Css,
                Some("(min-width: 1px)"),
            ),
            ("@media{}", Css, None),
            ("@font-face {}", Css, None),
            ("@include foo;", Scss, Some("foo")),
            ("@media #{$q} {}", Scss, Some("#{$q}")),
            ("@font-feature-values Fönt {}", Css, Some("Fönt")),
        ];
        for (src, syntax, expected) in cases {
            first_at_rule(src, *syntax, |a| {
                assert_eq!(
                    at_rule_prelude_span(src, a).map(|s| s.slice(src)),
                    *expected,
                    "{src:?}"
                );
            });
        }
        first_at_rule("@media all {}", Css, |a| {
            assert_eq!(at_rule_prelude_span("@media all {}", a), Some(sp(7, 10)))
        });
    }
}

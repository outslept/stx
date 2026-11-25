use std::collections::HashSet;

use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;

use crate::context::LintContext;
use crate::rules::Rule;

#[derive(Debug, Default)]
pub struct CustomPropertyNoMissingVarFunction;

pub fn rejected_message(custom_property: &str) -> String {
    format!("Missing var function for \"{custom_property}\"")
}

const PROPERTIES_ACCEPTING_DASHED_IDENT: &[&str] = &[
    "transition",
    "transition-property",
    "animation",
    "animation-name",
    "animation-timeline",
    "view-transition-name",
    "container",
    "container-name",
    "scroll-timeline",
    "scroll-timeline-name",
    "view-timeline",
    "view-timeline-name",
    "timeline-scope",
    "position-try",
    "position-try-fallbacks",
    "position-anchor",
    "anchor-name",
    "anchor-scope",
    "counter-reset",
    "counter-increment",
    "counter-set",
    "list-style",
    "list-style-type",
    "grid-row",
    "grid-row-start",
    "grid-row-end",
    "grid-column",
    "grid-column-start",
    "grid-column-end",
    "grid-area",
    "grid-template",
    "grid-template-areas",
    "grid-template-rows",
    "grid-template-columns",
    "will-change",
    "font-palette",
    "page",
];

fn is_property_accepting_dashed_ident(prop: &str) -> bool {
    PROPERTIES_ACCEPTING_DASHED_IDENT
        .iter()
        .any(|p| p.eq_ignore_ascii_case(prop))
}

#[derive(Debug)]
enum ValNode<'a> {
    Word {
        value: &'a str,
        offset: usize,
    },
    Div {
        value: &'a str,
    },
    Function {
        name: &'a str,
        args: Vec<ValNode<'a>>,
    },
}

impl<'a> ValNode<'a> {
    fn is_dashed_ident(&self) -> bool {
        matches!(self, ValNode::Word { value, .. } if value.starts_with("--"))
    }

    fn is_comma(&self) -> bool {
        matches!(self, ValNode::Div { value: "," })
    }
}

fn skip_parens(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
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
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                i += 1;
                break;
            }
        }
        i += 1;
    }
    i.min(len)
}

fn parse_value_nodes<'a>(s: &'a str, base_offset: usize) -> Vec<ValNode<'a>> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut nodes = Vec::new();

    while i < len {
        let b = bytes[i];

        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Comment
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }

        // String
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

        // Divs: `,`, `/`, `:`
        if b == b',' || b == b'/' || b == b':' {
            nodes.push(ValNode::Div {
                value: &s[i..i + 1],
            });
            i += 1;
            continue;
        }

        // Standalone `(`
        if b == b'(' {
            let open_paren = i;
            let close_paren = skip_parens(bytes, open_paren);
            let inner_end = if close_paren > open_paren + 1 && bytes[close_paren - 1] == b')' {
                close_paren - 1
            } else {
                close_paren
            };
            let args =
                parse_value_nodes(&s[open_paren + 1..inner_end], base_offset + open_paren + 1);
            nodes.push(ValNode::Function { name: "", args });
            i = close_paren;
            continue;
        }

        if b == b')' {
            i += 1;
            continue;
        }

        // Word or Function
        let start = i;
        while i < len {
            let c = bytes[i];
            if c == b'\\' && i + 1 < len {
                i += 2;
                continue;
            }
            if c.is_ascii_whitespace()
                || c == b','
                || c == b'/'
                || c == b':'
                || c == b'('
                || c == b')'
                || c == b'"'
                || c == b'\''
            {
                break;
            }
            i += 1;
        }

        if i < len && bytes[i] == b'(' {
            let func_name = &s[start..i];
            let open_paren = i;
            let close_paren = skip_parens(bytes, open_paren);
            let inner_end = if close_paren > open_paren + 1 && bytes[close_paren - 1] == b')' {
                close_paren - 1
            } else {
                close_paren
            };
            let args =
                parse_value_nodes(&s[open_paren + 1..inner_end], base_offset + open_paren + 1);
            nodes.push(ValNode::Function {
                name: func_name,
                args,
            });
            i = close_paren;
        } else if i > start {
            nodes.push(ValNode::Word {
                value: &s[start..i],
                offset: base_offset + start,
            });
        }
    }

    nodes
}

struct DeclRecord<'a> {
    prop: &'a str,
    value: &'a str,
    value_start: usize,
}

fn check_node<'a>(
    node: &ValNode<'a>,
    value_start: usize,
    known: &HashSet<String>,
    rule_name: &'static str,
    ctx: &mut LintContext<'a>,
) {
    match node {
        ValNode::Function { name, args } => {
            if name.eq_ignore_ascii_case("var") {
                for arg in args.iter().skip(1) {
                    check_node(arg, value_start, known, rule_name, ctx);
                }
                return;
            }

            if name.eq_ignore_ascii_case("running") {
                if let Some(ValNode::Function {
                    name: child_name,
                    args: child_args,
                }) = args.first()
                {
                    if child_name.eq_ignore_ascii_case("var") {
                        for arg in child_args.iter().skip(1) {
                            check_node(arg, value_start, known, rule_name, ctx);
                        }
                    }
                }
                return;
            }

            if name.eq_ignore_ascii_case("anchor") || name.eq_ignore_ascii_case("anchor-size") {
                let comma_index = args.iter().position(|n| n.is_comma());
                for (idx, arg) in args.iter().enumerate() {
                    let is_anchor_name =
                        arg.is_dashed_ident() && comma_index.map_or(true, |c| idx < c);
                    if !is_anchor_name {
                        check_node(arg, value_start, known, rule_name, ctx);
                    }
                }
                return;
            }

            if name.eq_ignore_ascii_case("style") {
                let mut found_colon = false;
                for arg in args {
                    if let ValNode::Div { value: ":" } = arg {
                        found_colon = true;
                    } else if !found_colon && arg.is_dashed_ident() {
                        // Skip the custom property name before `:` in style() query
                    } else if found_colon {
                        check_node(arg, value_start, known, rule_name, ctx);
                    }
                }
                return;
            }

            for arg in args {
                check_node(arg, value_start, known, rule_name, ctx);
            }
        }
        ValNode::Word { value, offset } => {
            if !value.starts_with("--") {
                return;
            }
            let clean_value = value.trim_end_matches(';');
            if !known.contains(clean_value) {
                return;
            }
            let start = (value_start + *offset) as u32;
            let end = start + clean_value.len() as u32;
            ctx.report(
                rule_name,
                Span { start, end },
                rejected_message(clean_value),
            );
        }
        ValNode::Div { .. } => {}
    }
}

impl Rule for CustomPropertyNoMissingVarFunction {
    fn name(&self) -> &'static str {
        "custom-property-no-missing-var-function"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let source = ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();

        let mut known_custom_properties: HashSet<String> = HashSet::new();
        let mut decls: Vec<DeclRecord<'a>> = Vec::new();
        let mut block_depth = 0usize;
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            if b == b'}' {
                block_depth = block_depth.saturating_sub(1);
                i += 1;
                continue;
            }

            // Comment
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                continue;
            }

            let stmt_start = i;
            let mut paren_depth = 0usize;
            let mut first_colon: Option<usize> = None;

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
                } else if paren_depth == 0 {
                    if c == b':' && first_colon.is_none() {
                        first_colon = Some(i);
                    } else if c == b'{' || c == b';' || c == b'}' {
                        break;
                    }
                }
                i += 1;
            }

            let stmt_slice = &source[stmt_start..i];

            if i < len && bytes[i] == b'{' {
                // Check if this is `@property --foo, --bar {}`
                if let Some(rest) = stmt_slice.strip_prefix('@') {
                    let name_len = rest
                        .bytes()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_')
                        .count();
                    let at_name = &rest[..name_len];
                    if at_name.eq_ignore_ascii_case("property") {
                        let params = rest[name_len..].trim();
                        for part in params.split(|c: char| c == ',' || c.is_ascii_whitespace()) {
                            let part = part.trim();
                            if part.starts_with("--") && part.len() > 2 {
                                known_custom_properties.insert(part.to_string());
                            }
                        }
                    }
                }
                block_depth += 1;
                i += 1;
            } else {
                // Declaration or statement ending in `;` or before `}`
                if block_depth > 0 && !stmt_slice.starts_with('@') {
                    if let Some(colon_pos) = first_colon {
                        let prop = source[stmt_start..colon_pos].trim();
                        if prop.starts_with("--") && prop.len() > 2 {
                            known_custom_properties.insert(prop.to_string());
                        }
                        let raw_val = &source[colon_pos + 1..i];
                        let trimmed_start = raw_val.len() - raw_val.trim_start().len();
                        let val_trimmed = raw_val.trim();
                        let val_start = colon_pos + 1 + trimmed_start;
                        decls.push(DeclRecord {
                            prop,
                            value: val_trimmed,
                            value_start: val_start,
                        });
                    }
                }
                if i < len && bytes[i] == b';' {
                    i += 1;
                }
            }
        }

        if known_custom_properties.is_empty() {
            return;
        }

        for decl in decls {
            if !decl.value.contains("--") {
                continue;
            }
            if is_property_accepting_dashed_ident(decl.prop) {
                continue;
            }
            let nodes = parse_value_nodes(decl.value, 0);
            for node in &nodes {
                check_node(
                    node,
                    decl.value_start,
                    &known_custom_properties,
                    self.name(),
                    ctx,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_warnings};

    const NAME: &str = "custom-property-no-missing-var-function";

    fn rejected(custom_property: &str) -> String {
        format!("Missing var function for \"{custom_property}\"")
    }

    #[test]
    fn custom_property_no_missing_var_function_default() {
        let pass = [
            "a { color: --foo; }",
            "a { color: var(--foo); }",
            "a { color: env(--foo); }",
            "a { color: color(--foo 0% 0% 0% 0%); }",
            "a { color: calc(var(--foo) + var(--bar)); }",
            "a { color: var(--foo, red); }",
            "a { color: var(--foo, var(--bar)); }",
            "a { --foo: var(--bar); }",
            ":root { --foo: red; } a { color: var(--foo); }",
            "@property --foo {} a { color: var(--foo); }",
            ":--foo {}",
            "@media(--foo) {}",
            "@property --foo {} a { transition: --foo; }",
            "@property --foo {} a { position: running(--foo); }",
            "@property --foo {} a { view-transition-name: --foo; }",
            "@property --qux {} a { container-name: --qux; }",
            "@property --foo {} a { scroll-timeline-name: --foo; }",
            "@property --foo {} a { SCROLL-TIMELINE-NAME: --foo; }",
            "a { color: calc(var(--foo) + 100px); }",
            "a { color: var(--foo, calc(var(--bar) + 20px)); }",
            "a { color: calc(100px + var(--foo)); }",
            "a { color: var(--foo, var(--bar, var(--foobar, #000))); }",
            "button { --state: normal; background-color: if(style(--state: hover): gray; else: white;); }",
            "@property --state {} button { background-color: if(style(--state: hover): gray; else: white;); }",
            ":root { --theme: dark; } a { color: if(style(--theme: light): black; else: white;); }",
            "a { --foo: 1; --bar: 2; color: if(style(--foo: 1): if(style(--bar: 2): red; else: blue;); else: green;); }",
            "a { --foo: 1; --bar: 2; color: if(style(--foo: var(--bar)): red; else: green;); }",
            "a { --foo: 1; --color: red; color: if(style(--foo: 1): var(--color); else: green;); }",
            "a { animation-timeline: --foo; }",
            "a { timeline-scope: --foo; }",
            "@property --foo {} a { position-try: most-width --foo; }",
            "@property --foo {} a { top: anchor(--foo bottom); }",
        ];
        let fail = [
            reject(
                "a { --foo: red; color: --foo; }",
                None,
                rejected("--foo"),
                (1, 24, 1, 29),
            ),
            reject(
                "@property --foo {} a { color: --foo; }",
                None,
                rejected("--foo"),
                (1, 31, 1, 36),
            ),
            reject(
                "@property --foo, --bar {} a { color: --bar; }",
                None,
                rejected("--bar"),
                (1, 38, 1, 43),
            ),
            reject(
                ":root { --bar: 0; } a { --foo: --bar; }",
                None,
                rejected("--bar"),
                (1, 32, 1, 37),
            ),
            reject(
                ":root { --bar: 0px; } a { color: calc(var(--foo) + --bar); }",
                None,
                rejected("--bar"),
                (1, 52, 1, 57),
            ),
            reject(
                ":root { --foo: pink; }  a { color: --foo, red; }",
                None,
                rejected("--foo"),
                (1, 36, 1, 41),
            ),
            reject(
                ":root { --bar: 0; } a { color: --foo(--bar); }",
                None,
                rejected("--bar"),
                (1, 38, 1, 43),
            ),
            reject_warnings(
                ":root {\n\t--bar: 0;\n\t--baz: 0;\n}\na {\n\t--foo: --bar;\n\tcolor: --baz;\n}",
                None,
                vec![
                    (rejected("--bar"), (6, 9, 6, 14)),
                    (rejected("--baz"), (7, 9, 7, 14)),
                ],
            ),
            reject_warnings(
                "@property --bar {}\n@property --baz {}\na {\n\t--foo: --bar;\n\tcolor: --baz;\n}",
                None,
                vec![
                    (rejected("--bar"), (4, 9, 4, 14)),
                    (rejected("--baz"), (5, 9, 5, 14)),
                ],
            ),
            reject(
                "a {\n\t--foo: red;\n\t--bar: blue;\n\tcolor: var(--foo, --bar);\n}",
                None,
                rejected("--bar"),
                (4, 20, 4, 25),
            ),
            reject(
                "a {\n\t--foo: qux;\n\t--bar: baz;\n\tposition: running(var(--foo, --bar));\n}",
                None,
                rejected("--bar"),
                (4, 31, 4, 36),
            ),
            reject(
                "a {\n\t--foo: red;\n\t--bar: blue;\n\t--baz: green;\n\tcolor: var(--foo, var(--bar, --baz));\n}",
                None,
                rejected("--baz"),
                (5, 31, 5, 36),
            ),
            reject(
                "a { --foo: 1; --bar: 2; color: if(style(--foo: --bar): red; else: green;); }",
                None,
                rejected("--bar"),
                (1, 48, 1, 53),
            ),
            reject(
                "a { --foo: 1; --color: red; color: if(style(--foo: 1): --color; else: green;); }",
                None,
                rejected("--color"),
                (1, 56, 1, 63),
            ),
            reject(
                "a { --foo: 10px; --bar: 0; top: anchor(--bar bottom, --foo); }",
                None,
                rejected("--foo"),
                (1, 54, 1, 59),
            ),
            reject(
                "a { --foo: 10px; --bar: 0; width: anchor-size(--bar width, --foo); }",
                None,
                rejected("--foo"),
                (1, 60, 1, 65),
            ),
            reject(
                "a { --foo: 10px; --bar: 0; top: anchor(--bar calc(50% + --foo)); }",
                None,
                rejected("--foo"),
                (1, 57, 1, 62),
            ),
        ];
        Tester::new(NAME, json!([true]), pass, fail).test();
    }

    #[test]
    #[ignore = "no effective language options in the legacy fixture"]
    fn custom_property_no_missing_var_function_language_options() {
        let pass = [":root { --foo: red; } a { color: --foo; }"];
        Tester::new(NAME, json!([true]), pass, []).test();
    }
}

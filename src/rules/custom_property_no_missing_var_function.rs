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

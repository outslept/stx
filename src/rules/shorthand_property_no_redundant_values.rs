use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        decl_value_raw, ident_name, is_standard_syntax_property, is_standard_syntax_value,
        secondary_str_list, unprefixed,
    },
};
use oxc_css_parser::ast::Declaration;
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Removes redundant values from supported shorthand properties.
    ///
    /// Set to `true`. Use `ignore: ["four-into-three-edge-values"]` to keep four values
    /// in directional shorthands when exactly three suffice. Auto-fix removes duplicates
    /// while keeping comments. Values with `var()` are not condensed; for slash-separated
    /// `border-radius`, this applies independently to each side of the slash.
    ///
    /// Example: `a { overflow: scroll scroll; }` is fixed to `a { overflow: scroll; }`.
    "shorthand-property-no-redundant-values", style, fix = fix;
    pub struct ShorthandPropertyNoRedundantValues {
        ignore_four_into_three: bool,
    }
}

impl ShorthandPropertyNoRedundantValues {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_four_into_three: secondary_str_list(secondary, "ignore")
                .iter()
                .any(|s| s == "four-into-three-edge-values"),
        }
    }
}

const SUPPORTED_SHORTHANDS: &[&str] = &[
    "margin",
    "margin-block",
    "margin-inline",
    "padding",
    "padding-block",
    "padding-inline",
    "border-color",
    "border-style",
    "border-width",
    "border-radius",
    "border-block-color",
    "border-block-style",
    "border-block-width",
    "border-inline-color",
    "border-inline-style",
    "border-inline-width",
    "gap",
    "grid-gap",
    "overflow",
    "overscroll-behavior",
    "scroll-margin",
    "scroll-margin-block",
    "scroll-margin-inline",
    "scroll-padding",
    "scroll-padding-block",
    "scroll-padding-inline",
    "inset",
    "inset-block",
    "inset-inline",
];

const FOUR_DIRECTIONAL_SHORTHAND_PROPERTIES: &[&str] = &[
    "margin",
    "padding",
    "border-color",
    "border-style",
    "border-width",
    "scroll-margin",
    "scroll-padding",
    "inset",
];

/// Top-level postcss-value-parser nodes; spaces and strings are dropped.
#[derive(Clone, Copy, PartialEq)]
enum Node<'a> {
    Word(&'a str),
    Function { raw: &'a str, var: bool },
    Div(u8),
    Comment(&'a str),
}

fn parse_nodes(value: &str) -> Vec<Node<'_>> {
    let bytes = value.as_bytes();
    let mut nodes = Vec::new();
    let mut i = 0;
    let skip_string = |mut j: usize| {
        let quote = bytes[j];
        j += 1;
        while j < bytes.len() && bytes[j] != quote {
            j += if bytes[j] == b'\\' { 2 } else { 1 };
        }
        (j + 1).min(bytes.len())
    };
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
        } else if value[i..].starts_with("/*") {
            let end = value[i + 2..]
                .find("*/")
                .map_or(bytes.len(), |e| i + 2 + e + 2);
            nodes.push(Node::Comment(&value[i..end]));
            i = end;
        } else if matches!(b, b'/' | b',' | b':') {
            nodes.push(Node::Div(b));
            i += 1;
        } else if matches!(b, b'"' | b'\'') {
            i = skip_string(i);
        } else {
            let start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && !matches!(bytes[i], b'/' | b',' | b':' | b'(' | b'"' | b'\'')
            {
                i += 1;
            }
            if bytes.get(i) == Some(&b'(') {
                let mut depth = 0usize;
                while i < bytes.len() {
                    match bytes[i] {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        b'"' | b'\'' => {
                            i = skip_string(i);
                            continue;
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let name = &value[start..value[start..].find('(').map_or(start, |p| start + p)];
                nodes.push(Node::Function {
                    raw: &value[start..i],
                    var: name.eq_ignore_ascii_case("var"),
                });
            } else if i == start {
                i += 1;
            } else {
                nodes.push(Node::Word(&value[start..i]));
            }
        }
    }
    nodes
}

fn can_condense<'a>(values: &[&'a str]) -> Vec<&'a str> {
    let get = |i: usize| values.get(i).copied().unwrap_or("");
    let (top, right, bottom, left) = (get(0), get(1), get(2), get(3));
    let [t, r, b, l] = [top, right, bottom, left].map(str::to_ascii_lowercase);
    let one = t == r && ((t == b && (b == l || l.is_empty())) || (b.is_empty() && l.is_empty()));
    let two = (t == b && r == l) || (t == b && l.is_empty() && t != r);
    let condensed = if one {
        vec![top]
    } else if two {
        vec![top, right]
    } else if r == l {
        vec![top, right, bottom]
    } else {
        vec![top, right, bottom, left]
    };
    condensed.into_iter().filter(|v| !v.is_empty()).collect()
}

/// The first values of each side of a slash, keeping comments in place.
fn condense_value(nodes: &[Node<'_>], counts: &[usize]) -> String {
    let mut tokens = Vec::new();
    let (mut side, mut kept) = (0, 0);
    for node in nodes {
        match *node {
            Node::Div(b'/') => {
                tokens.push("/");
                side += 1;
                kept = 0;
            }
            Node::Comment(raw) => tokens.push(raw),
            Node::Word(raw) | Node::Function { raw, .. }
                if kept < counts.get(side).copied().unwrap_or(0) =>
            {
                tokens.push(raw);
                kept += 1;
            }
            _ => {}
        }
    }
    tokens.join(" ")
}

fn value_of<'a>(node: &Node<'a>) -> Option<&'a str> {
    match *node {
        Node::Word(raw) | Node::Function { raw, .. } => Some(raw),
        _ => None,
    }
}

impl Rule for ShorthandPropertyNoRedundantValues {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        let Some((value, span)) = decl_value_raw(ctx.source, decl) else {
            return;
        };
        if !value.contains(|c: char| c.is_whitespace())
            || !ctx.is_standard_declaration(decl)
            || !is_standard_syntax_property(prop)
            || !is_standard_syntax_value(value)
        {
            return;
        }
        let lower = prop.to_ascii_lowercase();
        let normalized = unprefixed(&lower);
        if !SUPPORTED_SHORTHANDS.contains(&normalized) {
            return;
        }
        let nodes = parse_nodes(value);

        let (shortest, counts, values_form) =
            if normalized == "border-radius" && nodes.contains(&Node::Div(b'/')) {
                let mut sides: [Vec<&str>; 2] = [Vec::new(), Vec::new()];
                let mut has_var = [false; 2];
                let mut side = 0;
                for node in &nodes {
                    match *node {
                        Node::Div(b'/') if side == 0 => side = 1,
                        Node::Div(_) => return,
                        Node::Function { raw, var } => {
                            has_var[side] |= var;
                            sides[side].push(raw);
                        }
                        Node::Word(raw) => sides[side].push(raw),
                        Node::Comment(_) => {}
                    }
                }
                let condensed = [0, 1].map(|s| {
                    if has_var[s] {
                        sides[s].clone()
                    } else {
                        can_condense(&sides[s])
                    }
                });
                let shortest = format!("{} / {}", condensed[0].join(" "), condensed[1].join(" "))
                    .trim()
                    .to_string();
                let values_form = [sides[0].join(" "), sides[1].join(" ")].join(" / ");
                (
                    shortest,
                    vec![condensed[0].len(), condensed[1].len()],
                    values_form,
                )
            } else {
                if nodes
                    .iter()
                    .any(|node| matches!(node, Node::Div(_) | Node::Function { var: true, .. }))
                {
                    return;
                }
                let values: Vec<&str> = nodes.iter().filter_map(value_of).collect();
                if values.len() <= 1 || values.len() > 4 {
                    return;
                }
                let shortest = can_condense(&values);
                if self.ignore_four_into_three
                    && FOUR_DIRECTIONAL_SHORTHAND_PROPERTIES.contains(&normalized)
                    && values.len() == 4
                    && shortest.len() == 3
                {
                    return;
                }
                (shortest.join(" "), vec![shortest.len()], values.join(" "))
            };
        if shortest.eq_ignore_ascii_case(&values_form) {
            return;
        }
        ctx.report_with_fix(
            self.name(),
            span,
            format!("Expected \"{value}\" to be \"{shortest}\""),
            Some(Fix::single(span, condense_value(&nodes, &counts))),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message};
    use serde_json::json;

    #[test]
    fn condenses_like_stylelint() {
        let pass = [
            "a { margin: var(--margin) var(--margin); }",
            "a { margin: $variable $variable; }",
            "a { margin: 1px 2px 3px 4px; }",
        ];
        let fail = [
            reject_message(
                "a { margin: 1px /* foo */ 1px; }",
                "a { margin: 1px /* foo */; }",
                "Expected \"1px /* foo */ 1px\" to be \"1px\"",
            ),
            reject_message(
                "a { overflow: scroll scroll; }",
                "a { overflow: scroll; }",
                "Expected \"scroll scroll\" to be \"scroll\"",
            ),
            reject_message(
                "a { border-radius: 1px /* foo */ 1px / 2px 2px; }",
                "a { border-radius: 1px /* foo */ / 2px; }",
                "Expected \"1px /* foo */ 1px / 2px 2px\" to be \"1px / 2px\"",
            ),
            reject_message(
                "a { border-radius: 1px 1px / var(--foo) var(--foo); }",
                "a { border-radius: 1px / var(--foo) var(--foo); }",
                "Expected \"1px 1px / var(--foo) var(--foo)\" to be \"1px / var(--foo) var(--foo)\"",
            ),
        ];
        Tester::new(
            "shorthand-property-no-redundant-values",
            json!(true),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
        Tester::new(
            "shorthand-property-no-redundant-values",
            json!([true, {"ignore": ["four-into-three-edge-values"]}]),
            ["a { inset: auto 0 0 0; }"],
            [],
        )
        .test();
    }
}

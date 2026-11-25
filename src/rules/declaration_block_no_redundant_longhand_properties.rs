use std::{collections::HashMap, sync::OnceLock};

use oxc_css_parser::{
    Span,
    ast::{ComponentValue, Declaration, InterpolableIdent, SimpleBlock, Statement},
};
use serde_json::{Map, Value};

use crate::{
    context::{Fix, LintContext},
    rules::Rule,
    utils::{
        OptionMatcher, SHORTHAND_PROPERTIES, decl_value_raw, in_set_ignore_case,
        is_css_wide_keyword, is_descriptor_declaration, longhand_sub_properties_of_shorthand,
        options_matches, parse_option_matchers, secondary_str_list, vendor_prefix,
    },
};

const RULE_NAME: &str = "declaration-block-no-redundant-longhand-properties";

pub struct DeclarationBlockNoRedundantLonghandProperties {
    ignore_shorthands: Vec<OptionMatcher>,
    ignore_longhands: Vec<String>,
}

impl DeclarationBlockNoRedundantLonghandProperties {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_shorthands: parse_option_matchers(secondary, "ignoreShorthands"),
            ignore_longhands: secondary_str_list(secondary, "ignoreLonghands"),
        }
    }
}

struct Membership {
    shorthand: &'static str,
    len: usize,
    slot: usize,
}

fn memberships() -> &'static HashMap<&'static str, Vec<Membership>> {
    static MEMBERSHIPS: OnceLock<HashMap<&'static str, Vec<Membership>>> = OnceLock::new();
    MEMBERSHIPS.get_or_init(|| {
        let mut by_longhand = HashMap::<_, Vec<_>>::new();
        for &shorthand in SHORTHAND_PROPERTIES {
            let longhands = longhand_sub_properties_of_shorthand(shorthand).unwrap_or_default();
            for (slot, &longhand) in longhands.iter().enumerate() {
                by_longhand.entry(longhand).or_default().push(Membership {
                    shorthand,
                    len: longhands.len(),
                    slot,
                });
            }
        }
        by_longhand
    })
}

struct Longhand<'a> {
    decl: &'a Declaration<'a>,
    value: &'a str,
    index: usize,
    slot: usize,
}

#[derive(Default)]
struct Group<'a> {
    shorthand_value: Option<&'a str>,
    parts: Vec<Longhand<'a>>,
}

fn declaration_end(source: &str, end: u32) -> u32 {
    let mut rest = &source[end as usize..];
    loop {
        rest = rest.trim_ascii_start();
        if rest.starts_with(';') {
            return (source.len() - rest.len()) as u32 + 1;
        }
        let Some((_, after_comment)) = rest.strip_prefix("/*").and_then(|s| s.split_once("*/"))
        else {
            return end;
        };
        rest = after_comment;
    }
}

fn is_simple_box_value(decl: &Declaration<'_>, shorthand: &str) -> bool {
    let number = match decl.value.as_slice() {
        [ComponentValue::Number(number)] => return number.raw == "0",
        [ComponentValue::Dimension(dimension)]
            if in_set_ignore_case(&["px", "em"], dimension.unit.raw) =>
        {
            &dimension.value
        }
        [ComponentValue::Percentage(percent)] => &percent.value,
        [ComponentValue::InterpolableIdent(InterpolableIdent::Literal(ident))] => {
            return matches!(
                shorthand,
                "margin"
                    | "margin-block"
                    | "margin-inline"
                    | "inset"
                    | "inset-block"
                    | "inset-inline"
            ) && ident.raw.eq_ignore_ascii_case("auto");
        }
        _ => return false,
    };
    number.value.is_finite() && number.raw.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}

fn is_box_shorthand(shorthand: &str) -> bool {
    matches!(
        shorthand,
        "margin"
            | "margin-block"
            | "margin-inline"
            | "padding"
            | "padding-block"
            | "padding-inline"
            | "inset"
            | "inset-block"
            | "inset-inline"
            | "scroll-margin"
            | "scroll-margin-block"
            | "scroll-margin-inline"
            | "scroll-padding"
            | "scroll-padding-block"
            | "scroll-padding-inline"
    )
}

fn border_side_component(part: &Longhand<'_>, shorthand: &str) -> Option<usize> {
    let InterpolableIdent::Literal(ident) = &part.decl.name else {
        return None;
    };
    let suffix = ident
        .raw
        .to_ascii_lowercase()
        .strip_prefix(shorthand)?
        .strip_prefix('-')?
        .to_owned();
    match suffix.as_str() {
        "width" => Some(0),
        "style" => Some(1),
        "color" => Some(2),
        _ => None,
    }
}

fn is_simple_border_side_value(part: &Longhand<'_>, component: usize) -> bool {
    match (component, part.decl.value.as_slice()) {
        (0, [ComponentValue::Number(number)]) => number.raw == "0",
        (0, [ComponentValue::Dimension(dimension)]) => {
            in_set_ignore_case(&["px", "em"], dimension.unit.raw)
                && dimension.value.value.is_finite()
        }
        (1, [ComponentValue::InterpolableIdent(InterpolableIdent::Literal(style))]) => {
            in_set_ignore_case(
                &[
                    "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge",
                    "inset", "outset",
                ],
                style.raw,
            )
        }
        (2, [ComponentValue::InterpolableIdent(InterpolableIdent::Literal(_))]) => true,
        _ => false,
    }
}

fn shorthand_fix(
    ctx: &LintContext<'_>,
    shorthand: &str,
    parts: &[Longhand<'_>],
    span: Span,
) -> Option<Fix> {
    let grid = matches!(shorthand, "grid-row" | "grid-column");
    let border_side = matches!(
        shorthand,
        "border-top" | "border-right" | "border-bottom" | "border-left"
    );
    if !(grid || border_side || is_box_shorthand(shorthand))
        || is_descriptor_declaration(&ctx.ancestors)
        || !ctx.slice(span).is_ascii()
        || !ctx.comments_in_range(span.start, span.end).is_empty()
        || parts.windows(2).any(|pair| {
            ctx.slice_range(pair[0].decl.span.end, pair[1].decl.span.start)
                .trim_ascii()
                != ";"
        })
        || parts.iter().any(|part| {
            let (line, _) = ctx.line_index.line_col(ctx.source, part.decl.span.start);
            ctx.disables.is_suppressed(RULE_NAME, line)
                || if grid {
                    !part.value.parse::<i32>().is_ok_and(|n| n != 0)
                } else if border_side {
                    border_side_component(part, shorthand)
                        .is_none_or(|component| !is_simple_border_side_value(part, component))
                } else {
                    !is_simple_box_value(part.decl, shorthand)
                }
        })
    {
        return None;
    }

    let mut values = vec![""; if border_side { 3 } else { parts.len() }];
    for part in parts {
        let slot = if border_side {
            border_side_component(part, shorthand)?
        } else {
            part.slot
        };
        values[slot] = part.value;
    }
    let separator = if grid { " / " } else { " " };
    let mut replacement = format!("{shorthand}: {}", values.join(separator));
    if let Some(important) = &parts[0].decl.important {
        replacement.push(' ');
        replacement.push_str(ctx.slice(important.span));
    }
    Some(Fix::single(
        Span {
            start: span.start,
            end: parts.last()?.decl.span.end,
        },
        replacement,
    ))
}

impl Rule for DeclarationBlockNoRedundantLonghandProperties {
    fn name(&self) -> &'static str {
        RULE_NAME
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        if block.statements.len() < 2 {
            return;
        }
        let mut groups = HashMap::<String, Group<'a>>::new();

        for (index, statement) in block.statements.iter().enumerate() {
            let Statement::Declaration(decl) = statement else {
                continue;
            };
            let InterpolableIdent::Literal(ident) = &decl.name else {
                continue;
            };
            if ident.raw.starts_with("--")
                || decl.name_prefix.is_some()
                || decl.name_suffix.is_some()
                || decl.less_property_merge.is_some()
            {
                continue;
            }
            let Some((value, _)) = decl_value_raw(ctx.source, decl) else {
                continue;
            };
            let lower = ident.raw.to_ascii_lowercase();
            let prefix = vendor_prefix(&lower).unwrap_or_default();
            let property = &lower[prefix.len()..];
            let is_wide = is_css_wide_keyword(value);
            if longhand_sub_properties_of_shorthand(property).is_some() {
                let group = groups.entry(lower.clone()).or_default();
                group.shorthand_value = (!is_wide).then_some(value);
                group.parts.clear();
            }
            if is_wide
                || self
                    .ignore_longhands
                    .iter()
                    .any(|ignored| ignored == property)
            {
                continue;
            }
            let Some(memberships) = memberships().get(property) else {
                continue;
            };

            for &Membership {
                shorthand,
                len,
                slot,
            } in memberships
            {
                if options_matches(&self.ignore_shorthands, shorthand) {
                    continue;
                }
                let name = format!("{prefix}{shorthand}");
                let group = groups.entry(name.clone()).or_default();
                if group.shorthand_value == Some(value) {
                    ctx.report(
                        self.name(),
                        ident.span,
                        format!(
                            "Redundant longhand property \"{}\" after shorthand property \"{name}\"",
                            ident.raw
                        ),
                    );
                    break;
                }
                let parts = &mut group.parts;
                parts.push(Longhand {
                    decl,
                    value,
                    index,
                    slot,
                });
                if parts.len() != len
                    || parts.iter().enumerate().any(|(i, part)| {
                        part.decl.important.is_some() != decl.important.is_some()
                            || parts[..i].iter().any(|previous| previous.slot == part.slot)
                    })
                {
                    continue;
                }

                let first = &parts[0];
                let contiguous = index - first.index + 1 == len
                    && parts.windows(2).all(|pair| {
                        let end = declaration_end(ctx.source, pair[0].decl.span.end);
                        ctx.comments_in_range(end, pair[1].decl.span.start)
                            .is_empty()
                    });
                let span = Span {
                    start: if contiguous {
                        first.decl.span.start
                    } else {
                        decl.span.start
                    },
                    end: declaration_end(ctx.source, decl.span.end),
                };
                let fix = if contiguous && prefix.is_empty() {
                    shorthand_fix(ctx, shorthand, parts, span)
                } else {
                    None
                };
                ctx.report_with_fix(
                    self.name(),
                    span,
                    format!("Expected shorthand property \"{name}\""),
                    fix,
                );

                let consumed = std::mem::take(parts);
                for group in groups.values_mut() {
                    group.parts.retain(|part| {
                        !consumed.iter().any(|longhand| longhand.index == part.index)
                    });
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RULE_NAME;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn declaration_block_no_redundant_longhand_properties() {
        let pass = [
            "a { grid-row-start: 1; grid-row-end: 2 !important; }",
            "a { grid-row-start: 1; grid-row-start: 2; grid-row-end: 3; }",
            "a { -webkit-grid-row-start: 1; grid-row-end: 2; }",
            "a { grid-row-start: 1; & b { grid-row-end: 2; } }",
            "a { margin-left: inherit; margin-right: 1px; margin-top: 2px; margin-bottom: 3px; }",
            "a { background: red; background-color: blue; }",
        ];
        let fail = [
            reject_warnings(
                "a {\n\tgrid-row-start: 1;\n\tgrid-row-end: 2;\n\tgrid-column-start: 3;\n\tgrid-column-end: 4;\n}",
                "a {\n\tgrid-row: 1 / 2;\n\tgrid-column: 3 / 4;\n}",
                vec![
                    (
                        "Expected shorthand property \"grid-row\"".into(),
                        (2, 2, 3, 18),
                    ),
                    (
                        "Expected shorthand property \"grid-column\"".into(),
                        (4, 2, 5, 21),
                    ),
                ],
            ),
            reject(
                "a { -WEBKIT-transition-delay: 0.5s; -webKIT-transition-duration: 2s; -webkit-TRANSITION-property: top; -webkit-transition-timing-function: ease; }",
                None,
                "Expected shorthand property \"-webkit-transition\"",
                (1, 5, 1, 145),
            ),
            reject(
                "a {\n\tmargin-left: 40px;\n\tmargin-right: 10px;\n\tmargin-top: 20px;\n\tcolor: blue;\n\tmargin-bottom: 30px;\n}",
                None,
                "Expected shorthand property \"margin\"",
                (6, 2, 6, 22),
            ),
            reject(
                "a { background: red; background-color: red; }",
                None,
                "Redundant longhand property \"background-color\" after shorthand property \"background\"",
                (1, 22, 1, 38),
            ),
            reject(
                "a { margin: 1px; margin-left: 2px; margin-left: 1px !important; }",
                None,
                "Redundant longhand property \"margin-left\" after shorthand property \"margin\"",
                (1, 36, 1, 47),
            ),
        ];
        Tester::new(RULE_NAME, json!([true]), pass, fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn ignores_configured_shorthands_and_longhands() {
        Tester::new(
            RULE_NAME,
            json!([true, {"ignoreShorthands": ["/border/", "padding"], "ignoreLonghands": ["grid-row-end"]}]),
            [
                "a { border-top-width: 7px; border-top-style: double; border-top-color: green; }",
                "a { padding: 5px; padding-top: 5px; }",
                "a { grid-row-start: 1; grid-row-end: 2; }",
            ],
            [],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn declaration_block_no_redundant_longhand_properties_safe_fixes() {
        let fail = [
            reject(
                "a { MARGIN-LEFT: 10px; MARGIN-RIGHT: 10px; margin-top: 20px; margin-bottom: 30px; }",
                "a { margin: 20px 10px 30px 10px; }",
                "Expected shorthand property \"margin\"",
                (1, 5, 1, 82),
            ),
            reject(
                "a {\n  padding-left: 4px !important;\n  padding-top: 1px !important;\n  padding-right: 2px !important;\n  padding-bottom: 3px !important;\n  padding-bottom: 5px;\n}",
                "a {\n  padding: 1px 2px 3px 4px !important;\n  padding-bottom: 5px;\n}",
                "Expected shorthand property \"padding\"",
                (2, 3, 5, 34),
            ),
            reject(
                "a {\n  padding-top: var(--space);\n  padding-right: 2px;\n  padding-bottom: 3px;\n  padding-left: 4px;\n}",
                None,
                "Expected shorthand property \"padding\"",
                (2, 3, 5, 21),
            ),
            reject(
                "a { border-top-width: 7px; border-top-style: double; border-top-color: green; }",
                "a { border-top: 7px double green; }",
                "Expected shorthand property \"border-top\"",
                (1, 5, 1, 78),
            ),
            reject(
                "a { margin-left: 40px; margin-right: 10px; margin-top: 20px; margin-bottom: 30px; }",
                "a { margin: 20px 10px 30px 40px; }",
                "Expected shorthand property \"margin\"",
                (1, 5, 1, 82),
            ),
            reject(
                "a { margin-inline-start: 1px; margin-inline-end: 2px; }",
                "a { margin-inline: 1px 2px; }",
                "Expected shorthand property \"margin-inline\"",
                (1, 5, 1, 54),
            ),
            reject(
                "a { top: 1px; right: 2px; bottom: 3px; left: 4px; }",
                "a { inset: 1px 2px 3px 4px; }",
                "Expected shorthand property \"inset\"",
                (1, 5, 1, 50),
            ),
        ];
        Tester::new(RULE_NAME, json!([true]), [], fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn ambiguous_shorthands_are_not_fixed() {
        let fail = [
            reject(
                "a { font-style: italic; font-variant: normal; font-weight: bold; font-size: .8em; font-family: Arial; line-height: 1.2; font-stretch: normal; }",
                None,
                "Expected shorthand property \"font\"",
                (1, 5, 1, 142),
            ),
            reject(
                "a { grid-template-rows: var(--header-h) 1fr; grid-template-columns: 1fr; grid-template-areas: \"header header\"; }",
                None,
                "Expected shorthand property \"grid-template\"",
                (1, 5, 1, 111),
            ),
            reject(
                "a { grid-row-start: span 2; grid-row-end: 3; }",
                None,
                "Expected shorthand property \"grid-row\"",
                (1, 5, 1, 45),
            ),
            reject(
                "a { border-top-width: var(--w); border-top-style: solid; border-top-color: red; }",
                None,
                "Expected shorthand property \"border-top\"",
                (1, 5, 1, 80),
            ),
        ];
        Tester::new(RULE_NAME, json!([true]), [], fail)
            .with_fix(true)
            .test();
    }
}

use std::{collections::HashMap, sync::OnceLock};

use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent, SimpleBlock, Statement},
};
use serde_json::{Map, Value};

use crate::{
    context::{Fix, LintContext},
    fix::Edit,
    rules::Rule,
    utils::{
        OptionMatcher, SHORTHAND_PROPERTIES, decl_value_raw, is_css_wide_keyword,
        is_descriptor_declaration, longhand_sub_properties_of_shorthand, options_matches,
        parse_option_matchers, secondary_str_list, vendor_prefix,
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

/// postcss `decl.value`: comments removed, trimmed.
fn clean_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

fn comma_separated(value: Option<&str>) -> Vec<String> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0usize, 0);
    for (i, b) in value.bytes().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                parts.push(value[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim().to_string());
    parts.retain(|p| !p.is_empty());
    parts
}

/// Port of upstream `resolveShorthandValue`; `values` maps unprefixed longhands to cleaned values.
fn resolve_shorthand_value(
    name: &str,
    shorthand: &str,
    ordered: &[&str],
    values: &HashMap<&str, String>,
) -> Option<String> {
    let get = |p: &str| values.get(p).map(String::as_str).filter(|v| !v.is_empty());
    match name {
        "font" => {
            let [style, variant, weight, stretch, size, line_height, family] = [
                "font-style",
                "font-variant",
                "font-weight",
                "font-stretch",
                "font-size",
                "line-height",
                "font-family",
            ]
            .map(get);
            Some(format!(
                "{} {} {} {} {}/{} {}",
                style?, variant?, weight?, stretch?, size?, line_height?, family?
            ))
        }
        "font-synthesis" => {
            let parts = [
                ("font-synthesis-weight", "weight"),
                ("font-synthesis-style", "style"),
                ("font-synthesis-small-caps", "small-caps"),
            ];
            let mut auto = Vec::new();
            for (property, keyword) in parts {
                match get(property)? {
                    "auto" => auto.push(keyword),
                    "none" => {}
                    _ => return None,
                }
            }
            Some(if auto.is_empty() {
                "none".into()
            } else {
                auto.join(" ")
            })
        }
        "grid-column" | "grid-row" => Some(format!(
            "{} / {}",
            get(&format!("{name}-start"))?,
            get(&format!("{name}-end"))?
        )),
        "grid-template" => {
            let areas = get("grid-template-areas")?;
            let columns = get("grid-template-columns")?;
            let rows = get("grid-template-rows")?;
            if columns.contains("repeat(") || rows.contains("repeat(") {
                return None;
            }
            let split_areas: Vec<&str> = areas
                .split('"')
                .skip(1)
                .step_by(2)
                .filter(|area| !area.is_empty())
                .collect();
            let split_rows: Vec<&str> = rows.split(' ').collect();
            if split_areas.is_empty() || split_areas.len() != split_rows.len() {
                return None;
            }
            let zipped: Vec<String> = split_areas
                .iter()
                .zip(&split_rows)
                .map(|(area, row)| format!("\"{area}\" {row}"))
                .collect();
            Some(format!("{} / {columns}", zipped.join(" ")))
        }
        "transition" => {
            let [delays, durations, timings, properties] = [
                "transition-delay",
                "transition-duration",
                "transition-timing-function",
                "transition-property",
            ]
            .map(|p| comma_separated(get(p)));
            if delays.is_empty() || durations.is_empty() || timings.is_empty() {
                return None;
            }
            let items: Vec<String> = properties
                .iter()
                .enumerate()
                .map(|(i, property)| {
                    [
                        property,
                        &durations[i % durations.len()],
                        &timings[i % timings.len()],
                        &delays[i % delays.len()],
                    ]
                    .map(String::as_str)
                    .join(" ")
                })
                .collect();
            (!items.is_empty()).then(|| items.join(", "))
        }
        _ if shorthand == "background" && values.contains_key("background-size") => None,
        _ => Some(
            ordered
                .iter()
                .filter_map(|p| get(p))
                .collect::<Vec<_>>()
                .join(" "),
        ),
    }
}

/// Byte range postcss drops with `decl.remove()`: the whitespace before it through its `;`.
fn removal_span(source: &str, decl: &Declaration<'_>) -> Span {
    let before =
        source[..decl.span.start as usize].trim_end_matches(|c: char| c.is_ascii_whitespace());
    Span {
        start: before.len() as u32,
        end: declaration_end(source, decl.span.end),
    }
}

fn shorthand_fix(
    ctx: &LintContext<'_>,
    name: &str,
    shorthand: &str,
    ordered: &[&str],
    parts: &[Longhand<'_>],
) -> Option<Fix> {
    if is_descriptor_declaration(&ctx.ancestors)
        || parts.iter().any(|part| {
            let (line, _) = ctx.line_index.line_col(ctx.source, part.decl.span.start);
            ctx.disables.is_suppressed(RULE_NAME, line)
        })
    {
        return None;
    }
    let mut values = HashMap::new();
    for part in parts {
        let InterpolableIdent::Literal(ident) = &part.decl.name else {
            return None;
        };
        let lower = ident.raw.to_ascii_lowercase();
        let prefix = vendor_prefix(&lower).unwrap_or_default().len();
        let property = ordered.iter().find(|p| **p == &lower[prefix..])?;
        values.insert(*property, clean_value(part.value));
    }
    let value = resolve_shorthand_value(name, shorthand, ordered, &values)?;
    if value.is_empty() {
        return None;
    }
    let first = parts.iter().min_by_key(|part| part.index)?;
    let (_, value_span) = decl_value_raw(ctx.source, first.decl)?;
    let InterpolableIdent::Literal(ident) = &first.decl.name else {
        return None;
    };
    let mut edits = vec![
        Edit {
            span: ident.span,
            replacement: name.to_string(),
        },
        Edit {
            span: value_span,
            replacement: value,
        },
    ];
    // shortcut: when the removed last declaration had no `;`, postcss would also drop the new last one's.
    edits.extend(
        parts
            .iter()
            .filter(|part| part.index != first.index)
            .map(|part| Edit {
                span: removal_span(ctx.source, part.decl),
                replacement: String::new(),
            }),
    );
    Some(Fix { edits })
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
                let ignored = longhand_sub_properties_of_shorthand(shorthand)
                    .unwrap_or_default()
                    .iter()
                    .filter(|p| self.ignore_longhands.iter().any(|ignored| ignored == *p))
                    .count();
                let len = len - ignored;
                let name = format!("{prefix}{shorthand}");
                let group = groups.entry(name.clone()).or_default();
                if group.shorthand_value == Some(value) {
                    let (line, _) = ctx.line_index.line_col(ctx.source, decl.span.start);
                    let fix = (!ctx.disables.is_suppressed(RULE_NAME, line))
                        .then(|| Fix::single(removal_span(ctx.source, decl), ""));
                    ctx.report_with_fix(
                        self.name(),
                        ident.span,
                        format!(
                            "Redundant longhand property \"{}\" after shorthand property \"{name}\"",
                            ident.raw
                        ),
                        fix,
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
                let ordered: Vec<&str> = longhand_sub_properties_of_shorthand(shorthand)
                    .unwrap_or_default()
                    .iter()
                    .copied()
                    .filter(|p| !self.ignore_longhands.iter().any(|ignored| ignored == p))
                    .collect();
                let fix = shorthand_fix(ctx, &name, shorthand, &ordered, parts);
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
                "a { -webkit-transition: top 2s ease 0.5s; }",
                "Expected shorthand property \"-webkit-transition\"",
                (1, 5, 1, 145),
            ),
            reject(
                "a {\n\tmargin-left: 40px;\n\tmargin-right: 10px;\n\tmargin-top: 20px;\n\tcolor: blue;\n\tmargin-bottom: 30px;\n}",
                "a {\n\tmargin: 20px 10px 30px 40px;\n\tcolor: blue;\n}",
                "Expected shorthand property \"margin\"",
                (6, 2, 6, 22),
            ),
            reject(
                "a { background: red; background-color: red; }",
                "a { background: red; }",
                "Redundant longhand property \"background-color\" after shorthand property \"background\"",
                (1, 22, 1, 38),
            ),
            reject(
                "a { margin: 1px; margin-left: 2px; margin-left: 1px !important; }",
                "a { margin: 1px; margin-left: 2px; }",
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
            ],
            [reject(
                "a { grid-row-start: 1; grid-row-end: 2; }",
                None,
                "Expected shorthand property \"grid-row\"",
                (1, 5, 1, 23),
            )],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn declaration_block_no_redundant_longhand_properties_safe_fixes() {
        Tester::new(
            RULE_NAME,
            json!([true, {"ignoreLonghands": "text-decoration-thickness"}]),
            [],
            [reject(
                "a { text-decoration-line: underline; text-decoration-style: solid; text-decoration-color: purple; }",
                "a { text-decoration: underline solid purple; }",
                "Expected shorthand property \"text-decoration\"",
                (1, 5, 1, 98),
            )],
        )
        .with_fix(true)
        .test();
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
                "a {\n  padding: var(--space) 2px 3px 4px;\n}",
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
    fn custom_resolvers_and_unsplittable_grid_templates() {
        let fail = [
            reject(
                "a { font-style: italic; font-variant: normal; font-weight: bold; font-size: .8em; font-family: Arial; line-height: 1.2; font-stretch: normal; }",
                "a { font: italic normal bold normal .8em/1.2 Arial; }",
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
                "a { grid-row: span 2 / 3; }",
                "Expected shorthand property \"grid-row\"",
                (1, 5, 1, 45),
            ),
            reject(
                "a { border-top-width: var(--w); border-top-style: solid; border-top-color: red; }",
                "a { border-top: var(--w) solid red; }",
                "Expected shorthand property \"border-top\"",
                (1, 5, 1, 80),
            ),
        ];
        Tester::new(RULE_NAME, json!([true]), [], fail)
            .with_fix(true)
            .test();
    }
}

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{ValueTokenKind, is_standard_syntax_value, tokenize_value},
};
use oxc_css_parser::{Span, ast::Declaration};

const NAME: &str = "function-linear-gradient-no-nonstandard-direction";
const SIDES: [&str; 4] = ["top", "left", "bottom", "right"];

pub struct FunctionLinearGradientNoNonstandardDirection;

impl Rule for FunctionLinearGradientNoNonstandardDirection {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let span = Span {
            start: decl.colon_span.end,
            end: decl
                .important
                .as_ref()
                .map_or(decl.span.end, |imp| imp.span.start),
        };
        if !ctx.slice(span).contains('(') {
            return;
        }
        let tokens = tokenize_value(ctx.source, span);
        let mut url_depth = 0usize;
        for (i, token) in tokens.iter().enumerate() {
            if url_depth > 0 {
                if token.kind == ValueTokenKind::Function || token.raw == "(" {
                    url_depth += 1;
                } else if token.raw == ")" {
                    url_depth -= 1;
                }
                continue;
            }
            if token.kind != ValueTokenKind::Function {
                continue;
            }
            let name = token.raw.to_ascii_lowercase();
            let legacy = match name.as_str() {
                "linear-gradient(" => false,
                "-webkit-linear-gradient("
                | "-moz-linear-gradient("
                | "-o-linear-gradient("
                | "-ms-linear-gradient(" => true,
                "url(" => {
                    url_depth = 1;
                    continue;
                }
                _ => continue,
            };
            let mut depth = 0usize;
            let mut end = span.end;
            for arg in &tokens[i + 1..] {
                match arg.raw {
                    "," | ")" if depth == 0 => {
                        end = arg.span.start;
                        break;
                    }
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => depth = depth.saturating_sub(1),
                    _ if arg.kind == ValueTokenKind::Function => depth += 1,
                    _ => {}
                }
            }
            let start = token.span.end;
            let arg = ctx.slice_range(start, end).trim_end();
            if is_nonstandard(arg.trim_start(), legacy) {
                ctx.report(
                    NAME,
                    Span {
                        start,
                        end: start + arg.len() as u32,
                    },
                    "Nonstandard direction",
                );
            }
        }
    }
}

fn is_nonstandard(arg: &str, legacy: bool) -> bool {
    if !is_standard_syntax_value(arg) {
        return false;
    }
    let arg = arg.to_ascii_lowercase();
    if arg.starts_with("var(")
        || arg
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|word| word == "in")
    {
        return false;
    }
    let angle_char = |c: char| c.is_ascii_digit() || c == '.';
    if arg.starts_with(angle_char) {
        return !matches!(
            arg.trim_start_matches(angle_char),
            "deg" | "grad" | "rad" | "turn"
        );
    }
    if !SIDES.iter().any(|side| arg.contains(side)) {
        return false;
    }
    let direction = if legacy {
        arg.as_str()
    } else if let Some(direction) = arg.strip_prefix("to ") {
        direction
    } else {
        return true;
    };
    let mut sides = direction.split(' ');
    let first = sides.next().unwrap_or_default();
    !SIDES.contains(&first)
        || sides
            .next()
            .is_some_and(|second| second == first || !SIDES.contains(&second))
        || sides.next().is_some()
}

#[cfg(test)]
mod tests {
    use super::NAME;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn distinguishes_modern_and_legacy_directions_from_color_stops() {
        Tester::new(
            NAME,
            json!(true),
            [
                "a { x: -WebKit-linear-gradient(bottom right, red, blue); }",
                "a { x: linear-gradient(to right in oklab, red, blue); }",
                "a { x: linear-gradient(.25turn, red, blue); }",
                "a { x: linear-gradient(var(--top, red), blue); }",
                "a { x: linear-gradient(rgb(1, 2, 3), blue); }",
                "a { x: repeating-linear-gradient(bottom, red, blue); }",
            ],
            [
                reject(
                    "a { x: linear-gradient(  top  , red, blue); }",
                    None,
                    "Nonstandard direction",
                    (1, 24, 1, 29),
                ),
                reject(
                    "a { x: -ms-linear-gradient(to top, red, blue); }",
                    None,
                    "Nonstandard direction",
                    (1, 28, 1, 34),
                ),
                reject(
                    "a { x: linear-gradient(45px, red, blue); }",
                    None,
                    "Nonstandard direction",
                    (1, 24, 1, 28),
                ),
            ],
        )
        .test();
    }
}

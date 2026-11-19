use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::ComponentValue};

pub struct FunctionCalcNoUnspacedOperator;

impl Rule for FunctionCalcNoUnspacedOperator {
    fn name(&self) -> &'static str {
        "function-calc-no-unspaced-operator"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        if let ComponentValue::Function(func) = val {
            let raw = ctx.slice(func.span);
            let Some(paren_idx) = raw.find('(') else {
                return;
            };
            let func_name = raw[..paren_idx].trim();
            if !func_name.eq_ignore_ascii_case("calc")
                && !func_name.eq_ignore_ascii_case("-webkit-calc")
                && !func_name.eq_ignore_ascii_case("-moz-calc")
            {
                return;
            }

            let inner_start = func.span.start as usize + paren_idx + 1;
            let inner_end = (func.span.end as usize).saturating_sub(1);
            if inner_start >= inner_end || inner_end > ctx.source.len() {
                return;
            }

            let bytes = ctx.source.as_bytes();
            let mut i = inner_start;

            while i < inner_end {
                if bytes[i] == b'+' || bytes[i] == b'-' {
                    if bytes[i] == b'-'
                        && i + 1 < inner_end
                        && (bytes[i + 1] == b'-' || bytes[i + 1].is_ascii_alphabetic())
                    {
                        while i < inner_end
                            && (bytes[i].is_ascii_alphanumeric()
                                || bytes[i] == b'-'
                                || bytes[i] == b'_')
                        {
                            i += 1;
                        }
                        continue;
                    }

                    let prev_char = if i > inner_start {
                        Some(bytes[i - 1])
                    } else {
                        None
                    };
                    let next_char = if i + 1 < inner_end {
                        Some(bytes[i + 1])
                    } else {
                        None
                    };

                    let prev_is_operand = matches!(
                        prev_char,
                        Some(b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'%' | b')')
                    );
                    let next_is_operand = matches!(
                        next_char,
                        Some(b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'(' | b'.')
                    );

                    let prev_is_space = matches!(prev_char, Some(b' ' | b'\t' | b'\n' | b'\r'));
                    let next_is_space = matches!(next_char, Some(b' ' | b'\t' | b'\n' | b'\r'));

                    if (prev_is_operand && !next_is_space && next_is_operand)
                        || (prev_is_space && !next_is_space && next_is_operand)
                        || (prev_is_operand && next_is_space)
                    {
                        let op_ch = bytes[i] as char;
                        ctx.report(
                            self.name(),
                            Span {
                                start: i as u32,
                                end: (i + 1) as u32,
                            },
                            format!("Expected single spacearound \"{op_ch}\" operator"),
                        );
                    }
                }
                i += 1;
            }
        }
    }
}

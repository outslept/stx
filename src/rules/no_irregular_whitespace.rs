use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::Stylesheet};

pub struct NoIrregularWhitespace;

fn is_irregular_whitespace(ch: char) -> bool {
    matches!(
        ch,
        '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{00A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'
            ..='\u{200B}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

impl Rule for NoIrregularWhitespace {
    fn name(&self) -> &'static str {
        "no-irregular-whitespace"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        for (idx, ch) in ctx.source.char_indices() {
            if is_irregular_whitespace(ch) {
                let start = idx as u32;
                let end = start + ch.len_utf8() as u32;
                ctx.report(
                    self.name(),
                    Span { start, end },
                    "Unexpected irregular whitespace".to_string(),
                );
            }
        }
    }
}

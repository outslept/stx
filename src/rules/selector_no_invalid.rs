use crate::{context::LintContext, rules::Rule, utils::is_standard_syntax_rule};
use oxc_css_parser::ast::QualifiedRule;

pub struct SelectorNoInvalid;

fn invalid_reason(selector: &str) -> Option<&'static str> {
    let bytes = selector.as_bytes();
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut bracket_start = 0usize;
    let mut quote = 0u8;
    let mut i = 0usize;
    let mut comma_needs_selector = true;

    while i < bytes.len() {
        let byte = bytes[i];
        if quote != 0 {
            if byte == b'\\' {
                i += 2;
                continue;
            }
            if byte == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if byte == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        if byte == b'\\' {
            i += 2;
            comma_needs_selector = false;
            continue;
        }
        if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if byte == b'"' || byte == b'\'' {
            quote = byte;
            i += 1;
            continue;
        }
        match byte {
            b'[' => {
                brackets += 1;
                bracket_start = i;
                comma_needs_selector = false;
                i += 1;
                continue;
            }
            b']' => {
                if brackets == 0 {
                    return Some("unexpected input");
                }
                if brackets == 1 && selector[bracket_start + 1..i].trim().is_empty() {
                    return Some("identifier is expected");
                }
                brackets -= 1;
                i += 1;
                continue;
            }
            b'(' => {
                parens += 1;
                comma_needs_selector = false;
                i += 1;
                continue;
            }
            b')' if parens == 0 => return Some("unexpected input"),
            b')' => {
                parens -= 1;
                i += 1;
                continue;
            }
            b',' if brackets == 0 && parens == 0 => {
                if comma_needs_selector {
                    return Some("selector is expected");
                }
                comma_needs_selector = true;
                i += 1;
                continue;
            }
            _ => {}
        }
        if brackets > 0 {
            if byte.is_ascii_digit()
                && (i == 0 || bytes[i - 1] == b'[' || bytes[i - 1].is_ascii_whitespace())
            {
                return Some("identifier is expected");
            }
            if byte == b'=' && bytes.get(i + 1) == Some(&b'=') {
                return Some("identifier is expected");
            }
        } else {
            if byte == b'.'
                && (bytes.get(i + 1) == Some(&b'.') || !has_selector_ident(bytes, i + 1))
            {
                return Some("identifier is expected");
            }
            if byte == b'#' && !has_selector_ident(bytes, i + 1) {
                return Some("identifier is expected");
            }
            if byte == b':' {
                let name_start = i + if bytes.get(i + 1) == Some(&b':') {
                    2
                } else {
                    1
                };
                let name_end = selector_ident_end(bytes, name_start);
                if name_end == name_start {
                    return Some("identifier is expected");
                }
                let name = selector[name_start..name_end].to_ascii_lowercase();
                if name == "dir" && bytes.get(name_end) == Some(&b'(') {
                    let end = selector[name_end + 1..].find(')').map(|n| name_end + 1 + n);
                    let Some(end) = end else {
                        return Some("unexpected input");
                    };
                    let direction = selector[name_end + 1..end].trim();
                    if !direction.eq_ignore_ascii_case("ltr")
                        && !direction.eq_ignore_ascii_case("rtl")
                    {
                        return Some("expected \"ltr\" or \"rtl\" within \":dir()\"");
                    }
                }
            }
        }
        comma_needs_selector = false;
        i += 1;
    }

    if brackets > 0 || parens > 0 {
        return Some("unexpected input");
    }
    if let Some(reason) = special_invalid_reason(selector) {
        return Some(reason);
    }
    if comma_needs_selector && selector.contains(',') {
        Some("selector is expected")
    } else {
        None
    }
}

fn special_invalid_reason(selector: &str) -> Option<&'static str> {
    for (function, reason) in [
        (":not(", "pseudo-elements are invalid within \":not()\""),
        (":has(", "pseudo-elements are invalid within \":has()\""),
        (
            ":nth-child(",
            "pseudo-elements are invalid within \":nth-child()\"",
        ),
        (":host(", "pseudo-elements are invalid within \":host()\""),
        (
            "::slotted(",
            "pseudo-elements are invalid within \"::slotted()\"",
        ),
    ] {
        if function_argument(selector, function).is_some_and(|arguments| arguments.contains("::")) {
            return Some(reason);
        }
    }
    if function_argument(selector, ":has(")
        .is_some_and(|arguments| arguments.to_ascii_lowercase().contains(":has("))
    {
        return Some("\":has()\" is invalid within \":has()\"");
    }
    for (function, label) in [("::slotted(", "::slotted()"), (":host(", ":host()")] {
        if function_argument(selector, function).is_some_and(|arguments| {
            arguments.contains('>')
                || arguments.contains('+')
                || arguments.contains('~')
                || arguments.split_whitespace().count() > 1
        }) {
            return Some(if label == "::slotted()" {
                "combinators are invalid within \"::slotted()\""
            } else {
                "combinators are invalid within \":host()\""
            });
        }
    }
    let lower = selector.to_ascii_lowercase();
    if lower.contains("::after::before") {
        return Some("\"::before\" is invalid after \"::after\"");
    }
    if lower.contains("::after ")
        || lower.contains("::after>")
        || lower.contains("::after+")
        || lower.contains("::after~")
    {
        return Some("combinators are invalid after \"::after\"");
    }
    if lower.contains("::slotted(") && lower.contains(") ") {
        return Some("combinators are invalid after \"::slotted()\"");
    }
    None
}

fn function_argument<'a>(selector: &'a str, function: &str) -> Option<&'a str> {
    let lower = selector.to_ascii_lowercase();
    let start = lower.find(function)?;
    let open = start + function.len() - 1;
    let bytes = selector.as_bytes();
    let mut depth = 1usize;
    let mut quote = 0u8;
    let mut i = open + 1;
    while i < bytes.len() {
        if quote != 0 {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == quote {
                quote = 0;
            }
        } else if bytes[i] == b'\'' || bytes[i] == b'"' {
            quote = bytes[i];
        } else if bytes[i] == b'(' {
            depth += 1;
        } else if bytes[i] == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(&selector[open + 1..i]);
            }
        }
        i += 1;
    }
    None
}

fn has_selector_ident(bytes: &[u8], start: usize) -> bool {
    bytes.get(start).is_some_and(|byte| {
        byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'-' | b'\\') || *byte >= 0x80
    })
}

fn selector_ident_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i = (i + 2).min(bytes.len());
        } else if bytes[i].is_ascii_alphanumeric()
            || matches!(bytes[i], b'-' | b'_')
            || bytes[i] >= 0x80
        {
            i += 1;
        } else {
            break;
        }
    }
    i
}

impl Rule for SelectorNoInvalid {
    fn name(&self) -> &'static str {
        "selector-no-invalid"
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if !is_standard_syntax_rule(ctx.source, rule) {
            return;
        }
        for selector in &rule.selector.selectors {
            let raw = ctx.slice(selector.span).trim();
            if let Some(reason) = invalid_reason(raw) {
                ctx.report(
                    self.name(),
                    selector.span,
                    format!("Invalid selector \"{raw}\", {reason}"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject_message, reject_messages};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a {}",
            "a > b {}",
            ".foo.bar {}",
            "[foo=bar] {}",
            "[data-value='.. =='] {}",
            ":not(.foo) {}",
            ":dir(ltr) {}",
            "a, b {}",
            "a\\.b {}",
        ];
        let fail = vec![reject_message(
            ":dir(foo) {}",
            None,
            "Invalid selector \":dir(foo)\", expected \"ltr\" or \"rtl\" within \":dir()\"",
        )];
        Tester::new("selector-no-invalid", json!(true), &pass, &fail).test();
    }

    #[test]
    fn selector_grammar_regressions() {
        assert_eq!(invalid_reason(", a"), Some("selector is expected"));
        assert_eq!(invalid_reason("a ) b"), Some("unexpected input"));
        assert_eq!(invalid_reason(".foo..bar"), Some("identifier is expected"));
        assert_eq!(invalid_reason("[foo==bar]"), Some("identifier is expected"));
        assert_eq!(invalid_reason("[]"), Some("identifier is expected"));
        assert_eq!(
            invalid_reason(":not(::before)"),
            Some("pseudo-elements are invalid within \":not()\""),
        );
        assert_eq!(
            invalid_reason("::slotted(a .foo)"),
            Some("combinators are invalid within \"::slotted()\""),
        );
        assert_eq!(invalid_reason("[data='.. ==']"), None);
    }

    #[test]
    fn reports_each_invalid_selector_in_a_list() {
        let fail = [reject_messages(
            ":dir(foo), :DIR(bar) {}",
            None,
            vec![
                "Invalid selector \":dir(foo)\", expected \"ltr\" or \"rtl\" within \":dir()\""
                    .into(),
                "Invalid selector \":DIR(bar)\", expected \"ltr\" or \"rtl\" within \":dir()\""
                    .into(),
            ],
        )];
        Tester::new("selector-no-invalid", json!(true), [], &fail).test();
    }
}

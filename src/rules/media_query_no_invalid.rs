use crate::{context::LintContext, rules::Rule, utils::PatternMatcher};
use oxc_css_parser::{Span, ast::Stylesheet};

const RULE_NAME: &str = "media-query-no-invalid";

const REASON_CUSTOM: &str = "custom media queries can only be used in boolean queries";
const REASON_MIN_MAX_IN_RANGE: &str =
    "\"min-\" and \"max-\" prefixes are not needed when using range queries";
const REASON_MIN_MAX_IN_BOOLEAN: &str =
    "\"min-\" and \"max-\" prefixes are not needed in boolean queries";
const REASON_DISCRETE: &str = "discrete features can only be used in plain and boolean queries";

const RANGE_TYPE_MEDIA_FEATURE_NAMES: &[&str] = &[
    "any-hover",
    "any-pointer",
    "aspect-ratio",
    "color",
    "color-gamut",
    "color-index",
    "device-aspect-ratio",
    "device-height",
    "device-width",
    "height",
    "horizontal-viewport-segments",
    "monochrome",
    "resolution",
    "vertical-viewport-segments",
    "width",
];

#[derive(Default)]
pub struct MediaQueryNoInvalid {
    pub ignore_functions: Vec<PatternMatcher>,
}

impl MediaQueryNoInvalid {
    pub fn from_secondary(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        let mut ignore_functions = Vec::new();
        if let Some(arr) = secondary
            .and_then(|m| m.get("ignoreFunctions"))
            .and_then(|v| v.as_array())
        {
            for item in arr {
                if let Some(s) = item.as_str() {
                    let pattern = if s.starts_with('/') && (s.ends_with('/') || s.ends_with("/i")) {
                        s.to_string()
                    } else {
                        format!("^{}$", regex::escape(s))
                    };
                    ignore_functions.push(PatternMatcher::new(Some(&pattern)));
                }
            }
        }
        Self { ignore_functions }
    }

    fn has_ignored_function(&self, text: &str) -> bool {
        if self.ignore_functions.is_empty() {
            return false;
        }
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        while i < len {
            let b = bytes[i];
            if b.is_ascii_alphabetic() || b == b'_' || b == b'-' {
                let start = i;
                i += 1;
                while i < len
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
                {
                    i += 1;
                }
                if i < len && bytes[i] == b'(' {
                    let fn_name = &text[start..i];
                    if self.ignore_functions.iter().any(|m| m.is_match(fn_name)) {
                        return true;
                    }
                }
            } else {
                i += 1;
            }
        }
        false
    }

    fn report_invalid(
        &self,
        ctx: &mut LintContext<'_>,
        span: Span,
        query: &str,
        reason: Option<&str>,
    ) {
        let msg = match reason {
            Some(r) if !r.is_empty() => format!("Invalid media query \"{query}\", {r}"),
            _ => format!("Invalid media query \"{query}\""),
        };
        ctx.report(RULE_NAME, span, msg);
    }

    fn check_single_media_query(
        &self,
        src: &str,
        q_start: usize,
        q_end: usize,
        ctx: &mut LintContext<'_>,
    ) {
        let (q_start, q_end) = trim_bounds(src, q_start, q_end);
        if q_start >= q_end {
            return;
        }
        let q_str = &src[q_start..q_end];
        let q_span = Span {
            start: q_start as u32,
            end: q_end as u32,
        };

        let items = match lex_top_level_items(src, q_start, q_end) {
            Some(v) => v,
            None => {
                if !self.has_ignored_function(q_str) {
                    self.report_invalid(ctx, q_span, q_str, None);
                }
                return;
            }
        };

        if items.is_empty() {
            return;
        }

        let starts_as_condition = match &items[0] {
            TopItem::Paren { .. } => true,
            TopItem::Word { text, .. } if text.eq_ignore_ascii_case("not") => {
                matches!(items.get(1), Some(TopItem::Paren { .. }))
            }
            _ => false,
        };

        if starts_as_condition {
            self.validate_media_condition(src, &items, q_span, q_str, true, ctx);
            return;
        }

        let mut idx = 0;
        if let TopItem::Word { text, .. } = &items[idx]
            && (text.eq_ignore_ascii_case("not") || text.eq_ignore_ascii_case("only"))
        {
            idx += 1;
        }

        if idx >= items.len() {
            if !self.has_ignored_function(q_str) {
                self.report_invalid(ctx, q_span, q_str, None);
            }
            return;
        }

        match &items[idx] {
            TopItem::Word { text, .. } if is_valid_media_type(text) => {
                idx += 1;
            }
            _ => {
                if !self.has_ignored_function(q_str) {
                    self.report_invalid(ctx, q_span, q_str, None);
                }
                return;
            }
        }

        if idx == items.len() {
            return;
        }

        while idx < items.len() {
            match &items[idx] {
                TopItem::Word { text, .. } if text.eq_ignore_ascii_case("and") => {
                    idx += 1;
                }
                _ => {
                    if !self.has_ignored_function(q_str) {
                        self.report_invalid(ctx, q_span, q_str, None);
                    }
                    return;
                }
            }

            if idx >= items.len() {
                if !self.has_ignored_function(q_str) {
                    self.report_invalid(ctx, q_span, q_str, None);
                }
                return;
            }

            match &items[idx] {
                TopItem::Paren { start, end } => {
                    self.validate_media_in_parens(src, *start, *end, q_str, ctx);
                    idx += 1;
                }
                _ => {
                    if !self.has_ignored_function(q_str) {
                        self.report_invalid(ctx, q_span, q_str, None);
                    }
                    return;
                }
            }
        }
    }

    fn validate_media_condition(
        &self,
        src: &str,
        items: &[TopItem<'_>],
        container_span: Span,
        full_query_str: &str,
        allow_or: bool,
        ctx: &mut LintContext<'_>,
    ) {
        if items.is_empty() {
            if !self.has_ignored_function(full_query_str) {
                self.report_invalid(
                    ctx,
                    container_span,
                    &src[container_span.start as usize..container_span.end as usize],
                    None,
                );
            }
            return;
        }

        if let TopItem::Word { text, .. } = &items[0]
            && text.eq_ignore_ascii_case("not")
        {
            if items.len() == 2
                && let TopItem::Paren { start, end } = &items[1]
            {
                self.validate_media_in_parens(src, *start, *end, full_query_str, ctx);
                return;
            }
            if !self.has_ignored_function(full_query_str) {
                self.report_invalid(
                    ctx,
                    container_span,
                    &src[container_span.start as usize..container_span.end as usize],
                    None,
                );
            }
            return;
        }

        let mut idx = 0;
        let mut combinator: Option<&str> = None;

        loop {
            match items.get(idx) {
                Some(TopItem::Paren { .. }) => {
                    idx += 1;
                    if idx < items.len() {
                        match &items[idx] {
                            TopItem::Word { text, .. }
                                if text.eq_ignore_ascii_case("and")
                                    || (allow_or && text.eq_ignore_ascii_case("or")) =>
                            {
                                let cur_comb = if text.eq_ignore_ascii_case("and") {
                                    "and"
                                } else {
                                    "or"
                                };
                                if let Some(prev) = combinator {
                                    if prev != cur_comb {
                                        if !self.has_ignored_function(full_query_str) {
                                            self.report_invalid(
                                                ctx,
                                                container_span,
                                                &src[container_span.start as usize
                                                    ..container_span.end as usize],
                                                None,
                                            );
                                        }
                                        return;
                                    }
                                } else {
                                    combinator = Some(cur_comb);
                                }
                                idx += 1;
                                if idx >= items.len() {
                                    if !self.has_ignored_function(full_query_str) {
                                        self.report_invalid(
                                            ctx,
                                            container_span,
                                            &src[container_span.start as usize
                                                ..container_span.end as usize],
                                            None,
                                        );
                                    }
                                    return;
                                }
                            }
                            _ => {
                                if !self.has_ignored_function(full_query_str) {
                                    self.report_invalid(
                                        ctx,
                                        container_span,
                                        &src[container_span.start as usize
                                            ..container_span.end as usize],
                                        None,
                                    );
                                }
                                return;
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => {
                    if !self.has_ignored_function(full_query_str) {
                        self.report_invalid(
                            ctx,
                            container_span,
                            &src[container_span.start as usize..container_span.end as usize],
                            None,
                        );
                    }
                    return;
                }
            }
        }

        for item in items {
            if let TopItem::Paren { start, end } = item {
                self.validate_media_in_parens(src, *start, *end, full_query_str, ctx);
            }
        }
    }

    fn validate_media_in_parens(
        &self,
        src: &str,
        p_start: usize,
        p_end: usize,
        full_query_str: &str,
        ctx: &mut LintContext<'_>,
    ) {
        let p_span = Span {
            start: p_start as u32,
            end: p_end as u32,
        };
        let p_str = &src[p_start..p_end];
        let (inner_start, inner_end) = trim_bounds(src, p_start + 1, p_end - 1);
        if inner_start >= inner_end {
            if !self.has_ignored_function(full_query_str) {
                self.report_invalid(ctx, p_span, p_str, None);
            }
            return;
        }

        if let Some(inner_items) = lex_top_level_items(src, inner_start, inner_end) {
            let is_nested_cond = match inner_items.first() {
                Some(TopItem::Paren { .. }) => true,
                Some(TopItem::Word { text, .. }) if text.eq_ignore_ascii_case("not") => {
                    matches!(inner_items.get(1), Some(TopItem::Paren { .. }))
                }
                _ => false,
            };

            if is_nested_cond {
                self.validate_media_condition(src, &inner_items, p_span, full_query_str, true, ctx);
                return;
            }
        }

        let inner_str = &src[inner_start..inner_end];

        if let Some(colon_pos) = find_top_level_colon(inner_str) {
            let name = inner_str[..colon_pos].trim();
            let val = inner_str[colon_pos + 1..].trim();

            if !is_valid_feature_name(name) || val.is_empty() || !is_valid_plain_value(name, val) {
                if !self.has_ignored_function(full_query_str) {
                    self.report_invalid(ctx, p_span, p_str, None);
                }
                return;
            }

            if name.starts_with("--") {
                self.report_invalid(ctx, p_span, p_str, Some(REASON_CUSTOM));
            }
            return;
        }

        if let Some((left, op, right)) = split_range_feature(inner_str) {
            if !matches!(op, "<" | "<=" | ">" | ">=" | "=")
                || !is_valid_feature_name(left)
                || right.is_empty()
                || !is_valid_plain_value(left, right)
            {
                if !self.has_ignored_function(full_query_str) {
                    self.report_invalid(ctx, p_span, p_str, None);
                }
                return;
            }

            let name_lower = left.to_ascii_lowercase();
            if name_lower.starts_with("--") {
                self.report_invalid(ctx, p_span, p_str, Some(REASON_CUSTOM));
                return;
            }
            if name_lower.starts_with("min-") || name_lower.starts_with("max-") {
                self.report_invalid(ctx, p_span, p_str, Some(REASON_MIN_MAX_IN_RANGE));
                return;
            }
            if !RANGE_TYPE_MEDIA_FEATURE_NAMES
                .iter()
                .any(|&n| n.eq_ignore_ascii_case(&name_lower))
            {
                self.report_invalid(ctx, p_span, p_str, Some(REASON_DISCRETE));
                return;
            }
            return;
        }

        if is_valid_feature_name(inner_str) {
            let name_lower = inner_str.to_ascii_lowercase();
            if name_lower.starts_with("min-") || name_lower.starts_with("max-") {
                self.report_invalid(ctx, p_span, p_str, Some(REASON_MIN_MAX_IN_BOOLEAN));
            }
            return;
        }

        if !self.has_ignored_function(full_query_str) {
            self.report_invalid(ctx, p_span, p_str, None);
        }
    }
}

impl Rule for MediaQueryNoInvalid {
    fn name(&self) -> &'static str {
        RULE_NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        for (p_start, p_end) in scan_media_preludes(ctx.source) {
            for (q_start, q_end) in split_top_level_commas(ctx.source, p_start, p_end) {
                self.check_single_media_query(ctx.source, q_start, q_end, ctx);
            }
        }
    }
}

fn scan_media_preludes(src: &str) -> Vec<(usize, usize)> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if b == b'\'' || b == b'"' {
            let quote = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(len);
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
        if b == b'@' && i + 6 <= len && src[i + 1..i + 6].eq_ignore_ascii_case("media") {
            let after_name = i + 6;
            if after_name == len
                || bytes[after_name].is_ascii_whitespace()
                || matches!(bytes[after_name], b'(' | b'{' | b';' | b'/')
            {
                let mut j = after_name;
                let mut paren_depth = 0usize;
                while j < len {
                    let c = bytes[j];
                    if c == b'/' && j + 1 < len && bytes[j + 1] == b'*' {
                        j += 2;
                        while j + 1 < len && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                            j += 1;
                        }
                        j = (j + 2).min(len);
                        continue;
                    }
                    if c == b'\'' || c == b'"' {
                        let quote = c;
                        j += 1;
                        while j < len {
                            if bytes[j] == b'\\' {
                                j = (j + 2).min(len);
                                continue;
                            }
                            if bytes[j] == quote {
                                j += 1;
                                break;
                            }
                            j += 1;
                        }
                        continue;
                    }
                    match c {
                        b'(' => paren_depth += 1,
                        b')' => paren_depth = paren_depth.saturating_sub(1),
                        b'{' | b';' if paren_depth == 0 => break,
                        _ => {}
                    }
                    j += 1;
                }
                out.push((after_name, j));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }

    out
}

enum TopItem<'a> {
    Word { text: &'a str },
    Paren { start: usize, end: usize },
}

fn lex_top_level_items<'a>(src: &'a str, start: usize, end: usize) -> Option<Vec<TopItem<'a>>> {
    let bytes = src.as_bytes();
    let mut i = start;
    let mut items = Vec::new();

    while i < end {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && i + 1 < end && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(end);
            continue;
        }
        if bytes[i] == b'(' {
            let p_start = i;
            let mut depth = 1usize;
            i += 1;
            while i < end && depth > 0 {
                if bytes[i] == b'(' {
                    depth += 1;
                } else if bytes[i] == b')' {
                    depth -= 1;
                }
                i += 1;
            }
            if depth != 0 {
                return None;
            }
            items.push(TopItem::Paren {
                start: p_start,
                end: i,
            });
            continue;
        }
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'-' || bytes[i] == b'_' {
            let w_start = i;
            i += 1;
            while i < end
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            if i < end && bytes[i] == b'(' {
                return None;
            }
            items.push(TopItem::Word {
                text: &src[w_start..i],
            });
            continue;
        }
        return None;
    }

    Some(items)
}

fn split_top_level_commas(src: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur_start = start;
    let mut i = start;

    while i < end {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                out.push((cur_start, i));
                cur_start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push((cur_start, end));
    out
}

fn trim_bounds(src: &str, mut start: usize, mut end: usize) -> (usize, usize) {
    let bytes = src.as_bytes();
    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    (start, end)
}

fn is_valid_media_type(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if matches!(lower.as_str(), "only" | "not" | "and" | "or" | "layer") {
        return false;
    }
    name.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn is_valid_feature_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let bytes = name.as_bytes();
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'-' || bytes[0] == b'_') {
        return false;
    }
    bytes
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn find_top_level_colon(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b':' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn split_range_feature(s: &str) -> Option<(&str, &str, &str)> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut depth = 0usize;
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b'<' | b'>' | b'=' if depth == 0 => {
                let op_start = i;
                i += 1;
                while i < len && matches!(bytes[i], b'<' | b'>' | b'=') {
                    i += 1;
                }
                let op_end = i;
                let left = s[..op_start].trim();
                let op = &s[op_start..op_end];
                let right = s[op_end..].trim();
                return Some((left, op, right));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn is_valid_plain_value(feature_name: &str, val: &str) -> bool {
    if val.contains('(') {
        return false;
    }

    let name_lower = feature_name.to_ascii_lowercase();
    let base_name = name_lower
        .strip_prefix("min-")
        .or_else(|| name_lower.strip_prefix("max-"))
        .unwrap_or(&name_lower);

    !(matches!(
        base_name,
        "width" | "height" | "device-width" | "device-height"
    ) && val.ends_with('%'))
}

#[cfg(test)]
mod tests {
    use super::RULE_NAME;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    fn rejected(query: &str) -> String {
        format!("Invalid media query \"{query}\"")
    }

    fn rejected_with_reason(query: &str, reason: &str) -> String {
        format!("Invalid media query \"{query}\", {reason}")
    }

    #[test]
    fn default_config() {
        let pass = [
            "@media screen {}",
            "@media print {}",
            "@media all {}",
            "@media {}",
            "@media screen and (color) {}",
            "@media only screen and (color) {}",
            "@media speech and (device-aspect-ratio: 16/9) {}",
            "@media (width >= 600px) {}",
            "@media (width: 600px) {}",
            "@media not (width <= -100px) {}",
            "@media not (resolution: -300dpi) {}",
            "@media (min-width: 320.01px) {}",
            "@media not (color) {}",
            "@media (width < 600px) and (height < 600px) {}",
            "@media (width < 600px) and (height < 600px) {}",
            "@media (not (color)) or (hover) {}",
            "@media (not (color)) and (not (hover)) {}",
            "@media (not (color)) and (not (hover)) {}",
            "@media screen and (max-weight: 3kg) and (color), (color) {}",
            "@media not unknown {}",
        ];
        let fail = [
            reject("@media @foo {}", None, rejected("@foo"), (1, 8, 1, 12)),
            reject(
                "@media screen or (min-width > 500px) {}",
                None,
                rejected("screen or (min-width > 500px)"),
                (1, 8, 1, 37),
            ),
            reject(
                "@media ((min-width: 300px) and (hover: hover) or (aspect-ratio: 1 / 1)) {}",
                None,
                rejected("((min-width: 300px) and (hover: hover) or (aspect-ratio: 1 / 1))"),
                (1, 8, 1, 72),
            ),
            reject(
                "@media (min-width: var(--foo)) {}",
                None,
                rejected("(min-width: var(--foo))"),
                (1, 8, 1, 31),
            ),
            reject(
                "@media (min-width: 50%) {}",
                None,
                rejected("(min-width: 50%)"),
                (1, 8, 1, 24),
            ),
            reject(
                "@media ((color: 2) and (min-width: 50%)) {}",
                None,
                rejected("(min-width: 50%)"),
                (1, 24, 1, 40),
            ),
            reject_warnings(
                "@media ((color: foo(--bar)) and (min-width: 50%)) {}",
                None,
                vec![
                    (rejected("(color: foo(--bar))"), (1, 9, 1, 28)),
                    (rejected("(min-width: 50%)"), (1, 33, 1, 49)),
                ],
            ),
            reject_warnings(
                "@media (color: foo(--bar)), (min-width: 50%) {}",
                None,
                vec![
                    (rejected("(color: foo(--bar))"), (1, 8, 1, 27)),
                    (rejected("(min-width: 50%)"), (1, 29, 1, 45)),
                ],
            ),
            reject(
                "@media (--foo: 2) {}",
                None,
                rejected_with_reason(
                    "(--foo: 2)",
                    "custom media queries can only be used in boolean queries",
                ),
                (1, 8, 1, 18),
            ),
            reject(
                "@media (min-width < 500px) {}",
                None,
                rejected_with_reason(
                    "(min-width < 500px)",
                    "\"min-\" and \"max-\" prefixes are not needed when using range queries",
                ),
                (1, 8, 1, 27),
            ),
            reject(
                "@media (min-width) {}",
                None,
                rejected_with_reason(
                    "(min-width)",
                    "\"min-\" and \"max-\" prefixes are not needed in boolean queries",
                ),
                (1, 8, 1, 19),
            ),
            reject(
                "@media (grid < 0) {}",
                None,
                rejected_with_reason(
                    "(grid < 0)",
                    "discrete features can only be used in plain and boolean queries",
                ),
                (1, 8, 1, 18),
            ),
            reject(
                "@media (not (color) and (hover)) {}",
                None,
                rejected("(not (color) and (hover))"),
                (1, 8, 1, 33),
            ),
            reject(
                "@media (color) and (hover) or (width) {}",
                None,
                rejected("(color) and (hover) or (width)"),
                (1, 8, 1, 38),
            ),
            reject(
                "@media (width => 100px) {}",
                None,
                rejected("(width => 100px)"),
                (1, 8, 1, 24),
            ),
            reject(
                "@media (width == 100px) {}",
                None,
                rejected("(width == 100px)"),
                (1, 8, 1, 24),
            ),
            reject(
                "@media not(min-width: 100px) {}",
                None,
                rejected("not(min-width: 100px)"),
                (1, 8, 1, 29),
            ),
            reject(
                "@media foo screen {}",
                None,
                rejected("foo screen"),
                (1, 8, 1, 18),
            ),
            reject(
                "@media not screen foo (min-width: 300px) {}",
                None,
                rejected("not screen foo (min-width: 300px)"),
                (1, 8, 1, 41),
            ),
            reject(
                "@media (--foo: 300px) {}",
                None,
                rejected_with_reason(
                    "(--foo: 300px)",
                    "custom media queries can only be used in boolean queries",
                ),
                (1, 8, 1, 22),
            ),
            reject(
                "@media (--foo < 300px) {}",
                None,
                rejected_with_reason(
                    "(--foo < 300px)",
                    "custom media queries can only be used in boolean queries",
                ),
                (1, 8, 1, 23),
            ),
        ];

        Tester::new(RULE_NAME, json!([true]), pass, fail).test();
    }

    #[test]
    fn ignore_functions() {
        let pass = [
            "@media (min-width: theme(screens.md)) {}",
            "@media (min-width: theme(screens.md) and fooFunction(300px)) {}",
            "@media (min-width: getDefaultWidth()) {}",
            "@media (min-width: theme(screens.md) and var(300px)) {}",
            "@media (min-width: var(300px) or theme(screens.md)) {}",
            "@media theme(screens.md) {}",
            "@media (min-width: fooFunction(300px)), (min-width: theme(screens.md)) {}",
        ];
        let fail = [
            reject(
                "@media (min-width: var(--foo)) {}",
                None,
                rejected("(min-width: var(--foo))"),
                (1, 8, 1, 31),
            ),
            reject(
                "@media (min-width: var(--foo)) {}",
                None,
                rejected("(min-width: var(--foo))"),
                (1, 8, 1, 31),
            ),
            reject(
                "@media var(--foo) {}",
                None,
                rejected("var(--foo)"),
                (1, 8, 1, 18),
            ),
            reject(
                "@media (min-width: var(300px)), (min-width: theme(screens.md)) {}",
                None,
                rejected("(min-width: var(300px))"),
                (1, 8, 1, 31),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignoreFunctions": ["theme", "fooFunction", "/^get.*$/"] }]),
            pass,
            fail,
        )
        .test();
    }
}

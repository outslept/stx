//! Lexical checks and scans for CSS fragments.
//!
//! The `is_standard_syntax_*` predicates screen out preprocessor and template
//! constructs; they do not validate the CSS grammar. The URL, media feature, and
//! selector scanners return zero-based byte ranges relative to the input fragment,
//! with exclusive ends.

use std::borrow::Cow;

use crate::utils::scan::{ValueToken, ValueTokenKind, tokenize_value};

/// Returns `true` if `s` contains Less variable interpolation (`@{...}`).
#[inline]
pub fn has_less_interpolation(s: &str) -> bool {
    has_delimited(s, "@{", b'}')
}

/// Returns `true` if `s` contains SCSS/Sass interpolation (`#{...}`).
#[inline]
pub fn has_scss_interpolation(s: &str) -> bool {
    has_delimited(s, "#{", b'}')
}

/// Returns `true` if `s` contains PostCSS simple-vars interpolation (`$(...)`).
#[inline]
pub fn has_psv_interpolation(s: &str) -> bool {
    has_delimited(s, "$(", b')')
}

/// Returns `true` if `s` contains template literal or brace interpolation (`{...}`).
#[inline]
pub fn has_tpl_interpolation(s: &str) -> bool {
    has_delimited(s, "{", b'}')
}

/// Returns `true` if `s` contains any supported preprocessor or template interpolation.
///
/// These are delimiter checks, not a parser: they look for a closing delimiter
/// at least one byte past the first opening delimiter of each form. For example,
/// `.a{}` is not considered interpolated, but `.a{b}` is.
pub fn has_interpolation(s: &str) -> bool {
    has_less_interpolation(s)
        || has_scss_interpolation(s)
        || has_psv_interpolation(s)
        || has_tpl_interpolation(s)
}

fn has_delimited(s: &str, open: &str, close: u8) -> bool {
    s.find(open)
        .and_then(|i| s.as_bytes().get(i + open.len() + 1..))
        .is_some_and(|rest| rest.contains(&close))
}

/// Replaces contiguous runs of interpolation sigils (`#`, `@`, `{`, `}`) with `replacement`.
pub fn blur_interpolation(source: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_run = false;
    for ch in source.chars() {
        if matches!(ch, '#' | '@' | '{' | '}') {
            if !in_run {
                out.push_str(replacement);
                in_run = true;
            }
        } else {
            in_run = false;
            out.push(ch);
        }
    }
    out
}

/// Returns `true` if `property` is a SCSS variable (`$var`) or namespaced variable (`ns.$var`).
#[inline]
pub fn is_scss_variable(property: &str) -> bool {
    property.starts_with('$') || property.contains(".$")
}

/// Returns `true` if `prop` is a CSS custom property (`--*`).
#[inline]
pub fn is_custom_property(prop: &str) -> bool {
    prop.starts_with("--")
}

/// Returns `true` if `func` is a custom CSS function (`--*`).
#[inline]
pub fn is_custom_function(func: &str) -> bool {
    func.starts_with("--")
}

/// Returns `true` if `media_query` is a custom media query (`--*`).
#[inline]
pub fn is_custom_media_query(media_query: &str) -> bool {
    media_query.starts_with("--")
}

/// Returns `true` if `selector` is a custom selector (`:--*`).
#[inline]
pub fn is_custom_selector(selector: &str) -> bool {
    selector.starts_with(":--")
}

/// Returns `true` if `word` starts with `var(`, ignoring ASCII case.
/// Does not trim the input or check for a closing parenthesis.
///
/// # Panics
///
/// Panics if `word` is at least four bytes long and byte offset 4 is not a
/// UTF-8 character boundary.
#[inline]
pub fn is_variable(word: &str) -> bool {
    word.len() >= 4 && word[..4].eq_ignore_ascii_case("var(")
}

/// Returns `true` if `name` is an ASCII case-insensitive `url` function identifier.
#[inline]
pub fn is_url_function(name: &str) -> bool {
    name.eq_ignore_ascii_case("url")
}

/// Returns `true` if `name` is an ASCII case-insensitive `var` function identifier.
#[inline]
pub fn is_var_function(name: &str) -> bool {
    name.eq_ignore_ascii_case("var")
}

/// Returns `true` if `prop` is a standard CSS property name.
///
/// Rejects SCSS variables (`$foo`, `ns.$foo`), Less variables (`@foo`), Less property
/// merge syntax (`transform+`, `transform+_`), and interpolated identifiers.
pub fn is_standard_syntax_property(prop: &str) -> bool {
    if is_scss_variable(prop) || prop.starts_with('@') {
        return false;
    }
    if prop.ends_with('+') || prop.ends_with("+_") {
        return false;
    }
    !has_interpolation(prop)
}

/// Returns `true` if `value` is a standard CSS value token or expression.
///
/// Rejects preprocessor variables, namespaced SCSS members, interpolations, and
/// browser extension i18n placeholders (`__MSG_...__`).
pub fn is_standard_syntax_value(value: &str) -> bool {
    let mut normalized = value.trim();
    if normalized.is_empty() {
        return true;
    }
    if normalized.starts_with(['+', '-', '*', '/']) {
        normalized = &normalized[1..];
    }
    if normalized.starts_with('$') || normalized.starts_with('@') {
        return false;
    }
    if value.find(".$").is_some_and(|idx| idx > 0) {
        return false;
    }
    if has_interpolation(normalized) {
        return false;
    }
    if let Some(start) = value.find("__MSG_") {
        let rest = &value[start + 6..];
        if let Some(end) = rest.find("__")
            && end > 0
            && !rest[..end].chars().any(char::is_whitespace)
        {
            return false;
        }
    }
    if let Some(paren_idx) = value.find('(') {
        let before_paren = &value[..paren_idx];
        if let Some(dot_idx) = before_paren.rfind('.')
            && dot_idx > 0
        {
            let after_dot = &before_paren[dot_idx + 1..];
            let before_dot = &before_paren[..dot_idx];
            let before_char = before_dot.chars().next_back();
            if !after_dot.is_empty()
                && after_dot.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && after_dot
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                && before_char.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return false;
            }
        }
    }
    true
}

/// Returns `true` if `raw` is a standard CSS selector combinator rather than a reference
/// combinator such as `/deep/`.
#[inline]
pub fn is_standard_syntax_combinator(raw: &str) -> bool {
    let trimmed = raw.trim();
    !(trimmed.starts_with('/') || trimmed.ends_with('/'))
}

/// Returns `true` if `name` is a standard CSS function name.
#[inline]
pub fn is_standard_syntax_function(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with("#{")
        && !name.starts_with("${")
        && !name.starts_with('`')
        && !has_interpolation(name)
}

/// Returns `true` if `hex` is a standard hex color token rather than a Less map lookup
/// (e.g. `#colors[primary]`).
#[inline]
pub fn is_standard_syntax_hex_color(hex: &str) -> bool {
    !hex.contains('[')
}

/// Returns `true` if `keyframes_name` is free of preprocessor interpolation.
#[inline]
pub fn is_standard_syntax_keyframes_name(keyframes_name: &str) -> bool {
    !has_interpolation(keyframes_name)
}

/// Returns `true` if `selector` is a standard CSS selector string.
///
/// Rejects interpolations, SCSS placeholder selectors (`%foo`), nested property prefixes,
/// Less `:extend()` and parametric mixin definitions, ERB tags, and line comments.
pub fn is_standard_syntax_selector(selector: &str) -> bool {
    if has_interpolation(selector) {
        return false;
    }
    let trimmed = selector.trim();
    if trimmed.starts_with('%')
        || trimmed.ends_with(':')
        || trimmed.contains(":extend")
        || trimmed.contains("<%")
        || trimmed.contains("%>")
        || trimmed.contains("//")
    {
        return false;
    }
    if trimmed.ends_with(')') && !trimmed.contains(':') {
        return false;
    }
    if let Some(open_idx) = trimmed.rfind("(@")
        && trimmed.ends_with(')')
        && open_idx < trimmed.len() - 1
    {
        return false;
    }
    true
}

/// Returns `true` if `url` is a standard CSS `url(...)` argument free of preprocessor
/// variables and interpolations.
pub fn is_standard_syntax_url(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return true;
    }
    if has_scss_interpolation(url) || has_tpl_interpolation(url) || has_psv_interpolation(url) {
        return false;
    }
    if (url.starts_with('\'') && url.ends_with('\''))
        || (url.starts_with('"') && url.ends_with('"'))
    {
        return !has_less_interpolation(url);
    }
    if let Some(rest) = url.strip_prefix('@') {
        let rest = rest.strip_prefix('@').unwrap_or(rest);
        if !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return false;
        }
    }
    !(url.contains('$')
        && !url.ends_with('/')
        && url.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || b.is_ascii_whitespace()
                || matches!(
                    b,
                    b'$' | b'_'
                        | b'+'
                        | b'-'
                        | b','
                        | b'.'
                        | b'/'
                        | b'*'
                        | b'\''
                        | b'"'
                        | b'@'
                        | b'#'
                        | b'?'
                )
        }))
}

/// Returns `true` if `value` can be parsed as a finite floating-point number.
#[inline]
pub fn is_numbery(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && trimmed.parse::<f64>().is_ok_and(|n| n.is_finite())
}

/// Returns `true` if `value` is a valid 3-, 4-, 6-, or 8-digit hexadecimal color literal.
pub fn is_valid_hex(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Checks an identifier using ASCII name rules after trimming whitespace and
/// skipping hex and single-byte backslash escapes.
///
/// The remaining bytes must be ASCII letters, digits, `_`, or `-`, and must
/// not start with a digit or `-` followed by a digit. Unescaped non-ASCII
/// letters are rejected, so `foö` does not pass this check.
pub fn is_valid_identifier(ident: &str) -> bool {
    let s = ident.trim();
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    let mut cleaned = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            let mut hex_count = 0;
            while hex_count < 6
                && i + 1 + hex_count < bytes.len()
                && bytes[i + 1 + hex_count].is_ascii_hexdigit()
            {
                hex_count += 1;
            }
            if hex_count > 0 {
                i += 1 + hex_count;
                if i + 1 < bytes.len() && bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
                    i += 2;
                } else if i < bytes.len()
                    && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n' | b'\x0c')
                {
                    i += 1;
                }
                continue;
            }
            if !matches!(bytes[i + 1], b'\n' | b'\r') {
                i += 2;
                continue;
            }
        }
        cleaned.push(bytes[i]);
        i += 1;
    }
    if cleaned
        .iter()
        .any(|&b| !(b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
    {
        return false;
    }
    if cleaned.first().is_some_and(|b| b.is_ascii_digit()) {
        return false;
    }
    if cleaned.first() == Some(&b'-') && cleaned.get(1).is_some_and(|b| b.is_ascii_digit()) {
        return false;
    }
    true
}

/// Extracts the ASCII-lowercase scheme from a raw URL, without quotes or `url()`.
///
/// The scheme must start with an ASCII letter and may contain ASCII letters,
/// digits, `+`, `-`, or `.`. Requires `://` except for `data:`. For `http`,
/// `https`, `ws`, `wss`, and `ftp`, the text after `://` must be nonempty and
/// must not start with `/` or `:`. Returns `None` when these checks fail;
/// the rest of the URL is not validated.
pub fn get_scheme_from_url(url_str: &str) -> Option<String> {
    let colon_idx = url_str.find(':')?;
    if colon_idx == 0 {
        return None;
    }
    let scheme_raw = &url_str[..colon_idx];
    let mut chars = scheme_raw.chars();
    if !chars.next()?.is_ascii_alphabetic() {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
        return None;
    }
    let rest = &url_str[colon_idx + 1..];
    if !scheme_raw.eq_ignore_ascii_case("data") && !rest.starts_with("//") {
        return None;
    }
    if ["http", "https", "ws", "wss", "ftp"]
        .iter()
        .any(|s| scheme_raw.eq_ignore_ascii_case(s))
    {
        let after_slashes = &rest[2..];
        if after_slashes.is_empty()
            || after_slashes.starts_with('/')
            || after_slashes.starts_with(':')
        {
            return None;
        }
    }
    Some(scheme_raw.to_ascii_lowercase())
}

/// Splits a numeric word token into `(number_literal, unit_suffix)` after stripping
/// legacy IE backslash hacks (`\0`, `\9`) and interpolation sigils.
///
/// The suffix is the rest of the cleaned word, unchanged in case and not
/// validated as a CSS unit. It can be empty or `%`. Returns `None` for words
/// beginning with `#`, non-standard syntax tokens, or words without a leading number.
pub fn get_dimension(word: &str) -> Option<(String, String)> {
    if word.is_empty() || word.starts_with('#') || !is_standard_syntax_value(word) {
        return None;
    }
    let blurred: Cow<'_, str> = if word
        .bytes()
        .any(|b| matches!(b, b'\\' | b'#' | b'@' | b'{' | b'}'))
    {
        blur_interpolation(word, "")
            .replace("\\0", "")
            .replace("\\9", "")
            .into()
    } else {
        word.into()
    };
    let bytes = blurred.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut i = 0;
    if matches!(bytes[i], b'+' | b'-') {
        i += 1;
    }
    let mut has_digits = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digits = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        if i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                has_digits = true;
                i += 1;
            }
        } else if !has_digits {
            return None;
        }
    }
    if !has_digits {
        return None;
    }
    if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
        let mut exp_i = i + 1;
        if exp_i < bytes.len() && matches!(bytes[exp_i], b'+' | b'-') {
            exp_i += 1;
        }
        let exp_start = exp_i;
        while exp_i < bytes.len() && bytes[exp_i].is_ascii_digit() {
            exp_i += 1;
        }
        if exp_i > exp_start {
            i = exp_i;
        }
    }
    let number = blurred[..i].to_string();
    let unit = blurred[i..].to_string();
    Some((number, unit))
}

/// Returns the byte ranges of the arguments inside each `url(...)` call in `raw`.
///
/// Matches `url(` ASCII case-insensitively and skips calls with no closing
/// parenthesis. Each range excludes the name and enclosing parentheses but
/// preserves quotes and whitespace inside them. `url()` yields `(4, 4)`.
pub fn url_arguments(raw: &str) -> Vec<(usize, usize)> {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 4 <= len {
        if raw
            .get(i..i + 3)
            .is_some_and(|name| name.eq_ignore_ascii_case("url"))
            && bytes[i + 3] == b'('
            && (i == 0 || !is_url_ident_byte(bytes[i - 1]))
        {
            let start = i + 4;
            let mut j = start;
            let mut depth = 1usize;
            let mut quote = 0u8;
            while j < len {
                let b = bytes[j];
                if quote != 0 {
                    if b == b'\\' {
                        j += 2;
                        continue;
                    }
                    if b == quote {
                        quote = 0;
                    }
                } else if b == b'"' || b == b'\'' {
                    quote = b;
                } else if b == b'(' {
                    depth += 1;
                } else if b == b')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            if j < len {
                out.push((start, j));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[inline]
fn is_url_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80
}

/// Returns `(start, end)` byte ranges (relative to `params`) of media feature
/// names inside top-level `(...)` groups.
pub fn media_feature_names(params: &str) -> Vec<(usize, usize)> {
    let bytes = params.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < len {
        if bytes[i] != b'(' {
            i += 1;
            continue;
        }
        if i > 0 && is_media_ident_byte(bytes[i - 1]) {
            i += 1;
            continue;
        }
        let open = i;
        let mut j = i + 1;
        let mut depth = 1usize;
        let mut quote = 0u8;
        while j < len {
            let b = bytes[j];
            if quote != 0 {
                if b == b'\\' {
                    j += 2;
                    continue;
                }
                if b == quote {
                    quote = 0;
                }
            } else if b == b'"' || b == b'\'' {
                quote = b;
            } else if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            j += 1;
        }
        if j >= len {
            break;
        }
        let inner = &params[open + 1..j];
        let tokens = tokenize_value(
            inner,
            oxc_css_parser::Span {
                start: 0,
                end: inner.len() as u32,
            },
        );
        let mut top: Vec<&ValueToken<'_>> = Vec::new();
        let mut nested = 0usize;
        for token in &tokens {
            match token.kind {
                ValueTokenKind::Function => {
                    if nested == 0 {
                        top.push(token);
                    }
                    nested += 1;
                }
                ValueTokenKind::CloseParen => nested = nested.saturating_sub(1),
                _ => {
                    if nested == 0 {
                        top.push(token);
                    }
                }
            }
        }
        for (k, token) in top.iter().enumerate() {
            if token.kind != ValueTokenKind::Ident {
                continue;
            }
            let name = token.raw;
            if name.starts_with("--")
                || name.starts_with('$')
                || name.starts_with('@')
                || name.starts_with('#')
            {
                continue;
            }
            let prev = k.checked_sub(1).map(|idx| top[idx]);
            let next = top.get(k + 1).copied();
            let prev_range = prev.is_some_and(is_range_operator);
            let next_range = next.is_some_and(is_range_operator);
            let colon_form =
                prev.is_none() && next.is_some_and(|t| t.kind == ValueTokenKind::Colon);
            let boolean_form = prev.is_none() && next.is_none();
            if prev_range || next_range || colon_form || boolean_form {
                out.push((
                    open + 1 + token.span.start as usize,
                    open + 1 + token.span.end as usize,
                ));
            }
        }
        i = j + 1;
    }
    out
}

fn is_media_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80
}

fn is_range_operator(token: &ValueToken<'_>) -> bool {
    token.kind == ValueTokenKind::Other && matches!(token.raw, "<" | ">" | "=")
}

/// Tokens in one media feature group, with byte offsets relative to the media
/// parameters and exclusive range ends.
pub struct MediaFeatureGroup {
    /// Start of the feature name.
    pub name_start: usize,
    /// Exclusive end of the feature name.
    pub name_end: usize,
    /// `(start, end, unit_start)` for each dimension, where `unit_start` is the
    /// first byte after the numeric prefix.
    pub dimensions: Vec<(usize, usize, usize)>,
    /// `(start, end)` for identifiers other than the feature name, numbers,
    /// percentages, and dimensions. Dimensions appear in both lists.
    pub values: Vec<(usize, usize)>,
}

/// Finds feature groups in the raw media query parameters, without `@media` or its block.
///
/// Returns one entry per recognized parenthesized group, in source order. Each
/// entry uses the first name candidate in boolean, colon, or range form. Groups
/// without a candidate are skipped, as are tokens inside function arguments.
///
/// ```
/// use stx::utils::syntax::media_feature_groups;
///
/// let groups = media_feature_groups("(min-width: 100px)");
/// let group = &groups[0];
/// assert_eq!((group.name_start, group.name_end), (1, 10));
/// assert_eq!(group.dimensions, [(12, 17, 15)]);
/// assert_eq!(group.values, [(12, 17)]);
/// ```
pub fn media_feature_groups(params: &str) -> Vec<MediaFeatureGroup> {
    let bytes = params.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < len {
        if bytes[i] != b'(' || (i > 0 && is_media_ident_byte(bytes[i - 1])) {
            i += 1;
            continue;
        }
        let open = i;
        let mut j = i + 1;
        let mut depth = 1usize;
        let mut quote = 0u8;
        while j < len {
            let b = bytes[j];
            if quote != 0 {
                if b == b'\\' {
                    j += 2;
                    continue;
                }
                if b == quote {
                    quote = 0;
                }
            } else if b == b'"' || b == b'\'' {
                quote = b;
            } else if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            j += 1;
        }
        if j >= len {
            break;
        }
        let inner = &params[open + 1..j];
        let tokens = tokenize_value(
            inner,
            oxc_css_parser::Span {
                start: 0,
                end: inner.len() as u32,
            },
        );
        let mut top: Vec<&ValueToken<'_>> = Vec::new();
        let mut nested = 0usize;
        for token in &tokens {
            match token.kind {
                ValueTokenKind::Function => {
                    if nested == 0 {
                        top.push(token);
                    }
                    nested += 1;
                }
                ValueTokenKind::CloseParen => nested = nested.saturating_sub(1),
                _ => {
                    if nested == 0 {
                        top.push(token);
                    }
                }
            }
        }
        let mut name_index = None;
        for (idx, token) in top.iter().enumerate() {
            if token.kind != ValueTokenKind::Ident {
                continue;
            }
            let name = token.raw;
            if name.starts_with("--")
                || name.starts_with('$')
                || name.starts_with('@')
                || name.starts_with('#')
            {
                continue;
            }
            let prev = idx.checked_sub(1).map(|p| top[p]);
            let next = top.get(idx + 1).copied();
            let prev_range = prev.is_some_and(is_range_operator);
            let next_range = next.is_some_and(is_range_operator);
            let colon_form =
                prev.is_none() && next.is_some_and(|t| t.kind == ValueTokenKind::Colon);
            let boolean_form = prev.is_none() && next.is_none();
            if prev_range || next_range || colon_form || boolean_form {
                name_index = Some(idx);
                break;
            }
        }
        let Some(name_index) = name_index else {
            i = j + 1;
            continue;
        };
        let mut dimensions = Vec::new();
        let mut values = Vec::new();
        for (idx, token) in top.iter().enumerate() {
            if idx == name_index {
                continue;
            }
            match token.kind {
                ValueTokenKind::Dimension { num_len } => {
                    let start = open + 1 + token.span.start as usize;
                    let end = open + 1 + token.span.end as usize;
                    dimensions.push((start, end, start + num_len));
                    values.push((start, end));
                }
                ValueTokenKind::Ident | ValueTokenKind::Number | ValueTokenKind::Percentage => {
                    values.push((
                        open + 1 + token.span.start as usize,
                        open + 1 + token.span.end as usize,
                    ));
                }
                _ => {}
            }
        }
        let name_token = top[name_index];
        out.push(MediaFeatureGroup {
            name_start: open + 1 + name_token.span.start as usize,
            name_end: open + 1 + name_token.span.end as usize,
            dimensions,
            values,
        });
        i = j + 1;
    }
    out
}

/// Returns `(inner_start, inner_end)` byte ranges inside closed `[...]` attribute
/// blocks, excluding brackets and skipping blocks inside parentheses.
pub fn selector_attribute_blocks(selector: &str) -> Vec<(usize, usize)> {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < len {
        if bytes[i] == b'(' {
            i = skip_balanced_paren(selector, i);
            continue;
        }
        if bytes[i] == b'[' {
            let start = i + 1;
            let mut j = start;
            let mut quote = 0u8;
            while j < len {
                let b = bytes[j];
                if quote != 0 {
                    if b == b'\\' {
                        j += 2;
                        continue;
                    }
                    if b == quote {
                        quote = 0;
                    }
                } else if b == b'"' || b == b'\'' {
                    quote = b;
                } else if b == b']' {
                    break;
                }
                j += 1;
            }
            if j < len {
                out.push((start, j));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Returns `(start, end, normalized_value)` for each combinator in `selector`,
/// skipping parenthesized arguments, attribute blocks, and slash-delimited combinators.
/// The range covers only the operator or whitespace run. A descendant combinator
/// is normalized to one ASCII space.
pub fn selector_combinators(selector: &str) -> Vec<(usize, usize, String)> {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
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
        if b == b'(' {
            i = skip_balanced_paren(selector, i);
            continue;
        }
        if b == b'[' {
            let mut j = i + 1;
            let mut quote = 0u8;
            while j < len {
                let c = bytes[j];
                if quote != 0 {
                    if c == b'\\' {
                        j += 2;
                        continue;
                    }
                    if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = c;
                } else if c == b']' {
                    break;
                }
                j += 1;
            }
            i = (j + 1).min(len);
            continue;
        }
        if b == b'/' {
            if let Some(close) = selector[i + 1..].find('/') {
                i = i + 1 + close + 1;
                continue;
            }
            i += 1;
            continue;
        }
        if b == b'>' || b == b'+' || b == b'~' {
            out.push((i, i + 1, (b as char).to_string()));
            i += 1;
            continue;
        }
        if b == b'|' && i + 1 < len && bytes[i + 1] == b'|' {
            out.push((i, i + 2, "||".to_string()));
            i += 2;
            continue;
        }
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let end = i;
            let prev = selector[..start]
                .bytes()
                .rev()
                .find(|c| !c.is_ascii_whitespace());
            let next = selector[end..].bytes().find(|c| !c.is_ascii_whitespace());
            let prev_blocks =
                prev.is_some_and(|c| matches!(c, b'>' | b'+' | b'~' | b'|' | b'/' | b'('));
            let next_blocks =
                next.is_some_and(|c| matches!(c, b'>' | b'+' | b'~' | b'|' | b'/' | b')'));
            if prev.is_none() || next.is_none() || prev_blocks || next_blocks {
                continue;
            }
            out.push((start, end, " ".to_string()));
            continue;
        }
        i += 1;
    }
    out
}

fn skip_balanced_paren(selector: &str, open: usize) -> usize {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut i = open + 1;
    let mut depth = 1usize;
    let mut quote = 0u8;
    while i < len {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
        } else if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        }
        i += 1;
    }
    len
}

/// Returns `(value_start, value_end, name_start, name_end, is_element)` for each
/// pseudo selector outside parentheses and attribute blocks. `value` includes
/// the leading colon(s) but excludes arguments; `name` excludes the colons.
/// `is_element` is true only for `::`, not for legacy single-colon spellings.
pub fn selector_pseudos(selector: &str) -> Vec<(usize, usize, usize, usize, bool)> {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
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
        if b == b'(' {
            i = skip_balanced_paren(selector, i);
            continue;
        }
        if b == b'[' {
            let mut j = i + 1;
            let mut quote = 0u8;
            while j < len {
                let c = bytes[j];
                if quote != 0 {
                    if c == b'\\' {
                        j += 2;
                        continue;
                    }
                    if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = c;
                } else if c == b']' {
                    break;
                }
                j += 1;
            }
            i = (j + 1).min(len);
            continue;
        }
        if b == b':' {
            let is_element = i + 1 < len && bytes[i + 1] == b':';
            let name_start = i + if is_element { 2 } else { 1 };
            let mut j = name_start;
            while j < len {
                let c = bytes[j];
                if c == b'\\' {
                    j += 2;
                    continue;
                }
                if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c >= 0x80 {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > name_start {
                out.push((i, j, name_start, j, is_element));
                i = j;
                continue;
            }
            i = name_start;
            continue;
        }
        i += 1;
    }
    out
}

fn scan_selector<F: FnMut(usize, u8, &mut usize, &mut u8)>(selector: &str, mut f: F) {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    let mut in_attr = 0usize;
    let mut quote = 0u8;
    while i < len {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if in_attr > 0 {
            if b == b'"' || b == b'\'' {
                quote = b;
            } else if b == b'[' {
                in_attr += 1;
            } else if b == b']' {
                in_attr -= 1;
            }
            i += 1;
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
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == b'[' {
            in_attr += 1;
            i += 1;
            continue;
        }
        if b == b'"' || b == b'\'' {
            quote = b;
            i += 1;
            continue;
        }
        f(i, b, &mut in_attr, &mut quote);
        i += 1;
    }
}

/// Counts class occurrences in raw selector text, including inside pseudo arguments.
///
/// Skips comments, attribute blocks, quoted strings, and escaped dots. A dot
/// must be followed by a letter, hyphen, underscore, or non-ASCII byte.
pub fn count_selector_classes(selector: &str) -> usize {
    let bytes = selector.as_bytes();
    let mut count = 0;
    scan_selector(selector, |i, b, _, _| {
        if b == b'.'
            && i + 1 < bytes.len()
            && (bytes[i + 1].is_ascii_alphabetic()
                || bytes[i + 1] == b'-'
                || bytes[i + 1] == b'_'
                || bytes[i + 1] >= 0x80)
        {
            count += 1;
        }
    });
    count
}

/// Counts unescaped `*` bytes in raw selector text, including inside pseudo
/// arguments, but excluding comments, attribute blocks, and quoted strings.
pub fn count_selector_universals(selector: &str) -> usize {
    let mut count = 0;
    scan_selector(selector, |_, b, _, _| {
        if b == b'*' {
            count += 1;
        }
    });
    count
}

/// Returns the half-open byte ranges inside closed `[...]` blocks in `selector`,
/// including blocks inside pseudo arguments, in source order.
///
/// Unlike [`selector_attribute_blocks`], this scans inside parentheses.
pub fn selector_attribute_blocks_nested(selector: &str) -> Vec<(usize, usize)> {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut quote = 0u8;
    while i < len {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if b == b'"' || b == b'\'' {
            quote = b;
            i += 1;
            continue;
        }
        if b == b'[' {
            let start = i + 1;
            let mut j = start;
            let mut q = 0u8;
            while j < len {
                let c = bytes[j];
                if q != 0 {
                    if c == b'\\' {
                        j += 2;
                        continue;
                    }
                    if c == q {
                        q = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    q = c;
                } else if c == b']' {
                    break;
                }
                j += 1;
            }
            if j < len {
                out.push((start, j));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Returns `(start, end, normalized_value)` for combinators in raw selector
/// text, including inside parentheses, in source order.
///
/// The half-open byte range covers only the operator or the whitespace run.
/// Descendant whitespace normalizes to `" "`; slash-delimited combinators such
/// as `/deep/` are skipped.
pub fn selector_combinators_nested(selector: &str) -> Vec<(usize, usize, String)> {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut quote = 0u8;
    let mut attr = 0usize;
    while i < len {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if attr > 0 {
            if b == b'"' || b == b'\'' {
                quote = b;
            } else if b == b'[' {
                attr += 1;
            } else if b == b']' {
                attr -= 1;
            }
            i += 1;
            continue;
        }
        if b == b'[' {
            attr += 1;
            i += 1;
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
        if b == b'/' {
            if let Some(c) = selector[i + 1..].find('/') {
                i = i + 1 + c + 1;
                continue;
            }
            i += 1;
            continue;
        }
        if b == b'>' || b == b'+' || b == b'~' {
            out.push((i, i + 1, (b as char).to_string()));
            i += 1;
            continue;
        }
        if b == b'|' && i + 1 < len && bytes[i + 1] == b'|' {
            out.push((i, i + 2, "||".to_string()));
            i += 2;
            continue;
        }
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let end = i;
            let prev = selector[..start]
                .bytes()
                .rev()
                .find(|c| !c.is_ascii_whitespace());
            let next = selector[end..].bytes().find(|c| !c.is_ascii_whitespace());
            let prev_blocks =
                prev.is_some_and(|c| matches!(c, b'>' | b'+' | b'~' | b'|' | b'/' | b'('));
            let next_blocks =
                next.is_some_and(|c| matches!(c, b'>' | b'+' | b'~' | b'|' | b'/' | b')'));
            if prev.is_none() || next.is_none() || prev_blocks || next_blocks {
                continue;
            }
            out.push((start, end, " ".to_string()));
            continue;
        }
        i += 1;
    }
    out
}

/// Returns `(value_start, value_end, name_start, name_end, is_element)` for
/// pseudo selectors in source order, including those inside pseudo arguments.
///
/// The value range includes the leading colon(s); the name range excludes them.
/// Neither range includes arguments. `is_element` means the prefix was `::`,
/// so `:before` has `is_element == false`. This scan does not consume escapes
/// as part of pseudo names.
pub fn selector_pseudos_nested(selector: &str) -> Vec<(usize, usize, usize, usize, bool)> {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut quote = 0u8;
    let mut attr = 0usize;
    while i < len {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if attr > 0 {
            if b == b'"' || b == b'\'' {
                quote = b;
            } else if b == b'[' {
                attr += 1;
            } else if b == b']' {
                attr -= 1;
            }
            i += 1;
            continue;
        }
        if b == b'[' {
            attr += 1;
            i += 1;
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
        if b == b':' {
            let is_element = i + 1 < len && bytes[i + 1] == b':';
            let ns = i + if is_element { 2 } else { 1 };
            let mut j = ns;
            while j < len
                && (bytes[j].is_ascii_alphanumeric()
                    || bytes[j] == b'-'
                    || bytes[j] == b'_'
                    || bytes[j] >= 0x80)
            {
                j += 1;
            }
            if j > ns {
                out.push((i, j, ns, j, is_element));
                i = j;
                continue;
            }
            i = ns;
            continue;
        }
        i += 1;
    }
    out
}

/// Counts identifier-like type selectors in raw selector text, including inside
/// pseudo arguments. Namespace prefixes are not counted.
///
/// This scan recognizes explicit combinators but not descendant whitespace:
/// `a > b` counts as two, while `a b` counts as one.
pub fn count_selector_types(selector: &str) -> usize {
    let bytes = selector.as_bytes();
    let len = bytes.len();
    let mut count = 0;
    let mut prev_sig: u8 = 0;
    let mut i = 0usize;
    let mut quote = 0u8;
    let mut attr = 0usize;
    while i < len {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if attr > 0 {
            if b == b'"' || b == b'\'' {
                quote = b;
            } else if b == b'[' {
                attr += 1;
            } else if b == b']' {
                attr -= 1;
            }
            i += 1;
            continue;
        }
        if b == b'[' {
            attr += 1;
            prev_sig = b'[';
            i += 1;
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
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let ident_start = b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80;
        if ident_start {
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'-'
                    || bytes[i] == b'_'
                    || bytes[i] >= 0x80)
            {
                i += 1;
            }
            let next = if i < len { bytes[i] } else { 0 };
            let is_type = matches!(prev_sig, 0 | b'>' | b'+' | b'~' | b',' | b'(' | b'|');
            if is_type && next != b'(' && next != b'|' {
                count += 1;
            }
            prev_sig = b'a';
            continue;
        }
        prev_sig = b;
        i += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    type Group = (
        usize,
        usize,
        Vec<(usize, usize, usize)>,
        Vec<(usize, usize)>,
    );
    type Combo<'a> = (usize, usize, &'a str);
    type Pseudo = (usize, usize, usize, usize, bool);

    fn check(f: fn(&str) -> bool, cases: &[(&str, bool)]) {
        for &(input, want) in cases {
            assert_eq!(f(input), want, "{input:?}");
        }
    }

    fn groups(params: &str) -> Vec<Group> {
        media_feature_groups(params)
            .into_iter()
            .map(|g| (g.name_start, g.name_end, g.dimensions, g.values))
            .collect()
    }

    fn combos(v: &[Combo]) -> Vec<(usize, usize, String)> {
        v.iter().map(|&(s, e, c)| (s, e, c.to_string())).collect()
    }

    #[test]
    fn has_interpolation_upstream() {
        check(
            has_interpolation,
            &[
                ("(min-width#{$value}: 10px)", true),
                ("(@{value}min-width : 10px)", true),
                ("#{$Attr}-color", true),
                ("@{Attr}-color", true),
                ("#{50% - $n}", true),
                (".n-#{$n}", true),
                (":n-#{$n}", true),
                (".n-@{n}", true),
                ("(min-width: 10px)", false),
                (".a{}", false),
                ("$sass-variable + 'foo'", false),
                ("10px", false),
                ("@less-variable + 'foo'", false),
                ("${value}", true),
                ("${\nvalue\n}", true),
            ],
        );
    }

    #[test]
    fn has_flavored_interpolation() {
        check(
            has_less_interpolation,
            &[
                ("@{a}", true),
                ("x-@{a}-y", true),
                ("@{}", false),
                ("#{a}", false),
                ("@a", false),
            ],
        );
        check(
            has_scss_interpolation,
            &[
                ("#{$a}", true),
                (".n-#{$n}", true),
                ("#{}", false),
                ("@{a}", false),
                ("#a", false),
            ],
        );
        check(
            has_psv_interpolation,
            &[
                ("$(a)", true),
                ("x$(a)y", true),
                ("$()", false),
                ("$a", false),
                ("(a)", false),
            ],
        );
        check(
            has_tpl_interpolation,
            &[
                ("{a}", true),
                ("${a}", true),
                ("{\na}", true),
                ("{}", false),
                ("{a", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn blur_interpolation_upstream() {
        let cases = [
            ("#{$selector}", " ", " $selector "),
            ("#{$selector}", "`", "`$selector`"),
            ("#{$selector * 10px}", " ", " $selector * 10px "),
            (
                "#{$font-size}/#{$line-height}",
                " ",
                " $font-size / $line-height ",
            ),
            ("url(#{$selector * 10px})", " ", "url( $selector * 10px )"),
            ("calc(#{$selector} * 2)", " ", "calc( $selector  * 2)"),
            (
                "filter: progid:DXImageTransform.Microsoft.gradient(enabled='false', startColorstr='#{ie-hex-str($green)}', endColorstr='#{ie-hex-str($translucent-red)}');",
                " ",
                "filter: progid:DXImageTransform.Microsoft.gradient(enabled='false', startColorstr=' ie-hex-str($green) ', endColorstr=' ie-hex-str($translucent-red) ');",
            ),
            ("\"I ate #{5 + 10} pies!\"", " ", "\"I ate  5 + 10  pies!\""),
            ("@{variable}", " ", " variable "),
        ];
        for (input, rep, want) in cases {
            assert_eq!(blur_interpolation(input, rep), want, "{input:?}");
        }
    }

    #[test]
    fn is_scss_variable_upstream() {
        check(
            is_scss_variable,
            &[
                ("$sass-variable", true),
                ("namespace.$sass-variable", true),
                ("#{$Attr}-color", false),
                ("top", false),
                ("border-top-left-radius", false),
                ("-webkit-appearance", false),
                ("--custom-property", false),
                ("@var", false),
                ("transform+", false),
                ("transform+_", false),
            ],
        );
    }

    #[test]
    fn is_custom_property_upstream() {
        check(
            is_custom_property,
            &[
                ("--custom-property", true),
                ("border-top-left-radius", false),
                ("-webkit-appearance", false),
                ("$sass-variable", false),
                ("@less-variable", false),
                ("var(--something)", false),
                ("var(  --something  )", false),
            ],
        );
    }

    #[test]
    fn is_custom_function_upstream() {
        check(
            is_custom_function,
            &[
                ("--custom-function", true),
                ("calc", false),
                ("$sass-variable", false),
                ("@less-variable", false),
                ("var(--something)", false),
                ("var(  --something  )", false),
            ],
        );
    }

    #[test]
    fn is_custom_media_query_upstream() {
        check(
            is_custom_media_query,
            &[
                ("--custom-media-query", true),
                ("border-top-left-radius", false),
                ("-webkit-appearance", false),
                ("$sass-variable", false),
                ("@less-variable", false),
                ("var(--something)", false),
                ("var(  --something  )", false),
            ],
        );
    }

    #[test]
    fn is_custom_selector_upstream() {
        check(
            is_custom_selector,
            &[
                (":--custom-selector", true),
                ("a", false),
                ("#div", false),
                (":hover", false),
                (":before", false),
                ("::before", false),
            ],
        );
    }

    #[test]
    fn is_variable_upstream() {
        check(
            is_variable,
            &[
                ("var(--something)", true),
                ("vAr(--something)", true),
                ("VAR(--something)", true),
                ("var(  --something  )", true),
                ("initial", false),
                ("currentColor", false),
                ("-webkit-appearance", false),
                ("--custom-property", false),
                ("$sass-variable", false),
                ("@less-variable", false),
            ],
        );
    }

    #[test]
    fn is_url_function_upstream() {
        check(
            is_url_function,
            &[
                ("calc", false),
                ("calc", false),
                ("rem", false),
                ("var", false),
                ("round", false),
                ("URL", true),
                ("url", true),
                ("url", true),
            ],
        );
    }

    #[test]
    fn is_var_function_cases() {
        check(
            is_var_function,
            &[
                ("var", true),
                ("VAR", true),
                ("vAr", true),
                ("variable", false),
                ("--var", false),
                ("var(", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_standard_syntax_property_upstream() {
        check(
            is_standard_syntax_property,
            &[
                ("top", true),
                ("--custom-property", true),
                ("border-top-left-radius", true),
                ("-webkit-appearance", true),
                ("$sass-variable", false),
                ("namespace.$sass-variable", false),
                ("#{$Attr}-color", false),
                ("@{Attr}-color", false),
                ("transform+", false),
                ("transform+_", false),
            ],
        );
    }

    #[test]
    fn is_standard_syntax_value_upstream() {
        check(
            is_standard_syntax_value,
            &[
                ("initial", true),
                ("currentColor", true),
                ("10px", true),
                ("45deg", true),
                ("$sass-variable", false),
                ("namespace.$sass-variable", false),
                ("-$sass-variable", false),
                ("+$sass-variable", false),
                ("@less-variable", false),
                ("-@less-variable", false),
                ("#{$var}", false),
                ("-#{$var}", false),
                ("@{var}", false),
                ("__MSG_@@bidi_dir__", false),
                ("__msg_@@bidi_dir__", true),
            ],
        );
    }

    #[test]
    fn is_standard_syntax_combinator_upstream() {
        check(
            is_standard_syntax_combinator,
            &[
                (" ", true),
                ("\t", true),
                ("\n", true),
                (" >> ", true),
                (" > ", true),
                (" + ", true),
                (" ~ ", true),
                (" /for/ ", false),
                (" /fOr/ ", false),
                (" /FOR/ ", false),
            ],
        );
    }

    #[test]
    fn is_standard_syntax_function_upstream() {
        check(
            is_standard_syntax_function,
            &[
                ("calc", true),
                ("url", true),
                ("", false),
                ("", false),
                ("#{darken", false),
                ("${", false),
                ("`calc", false),
            ],
        );
    }

    #[test]
    fn is_standard_syntax_hex_color_upstream() {
        check(
            is_standard_syntax_hex_color,
            &[("#000000", true), ("#prop[someprop]", false)],
        );
    }

    #[test]
    fn is_standard_syntax_keyframes_name_upstream() {
        check(
            is_standard_syntax_keyframes_name,
            &[
                ("slidein", true),
                ("slide-in", true),
                ("slide_in", true),
                ("frame-#{$name}", false),
                ("#{$name}-frame", false),
                ("'frame-#{$name}'", false),
                ("'#{$name}-frame'", false),
                ("\"frame-#{$name}\"", false),
                ("\"#{$name}-frame\"", false),
                ("frame-@{name}", false),
                ("@{name}-frame", false),
                ("'frame-@{name}'", false),
                ("'@{name}-frame'", false),
                ("\"frame-@{name}\"", false),
                ("\"@{name}-frame\"", false),
            ],
        );
    }

    #[test]
    fn is_standard_syntax_selector_upstream() {
        check(
            is_standard_syntax_selector,
            &[
                ("a", true),
                (".a", true),
                ("[a=a]", true),
                ("*", true),
                ("a:last-child", true),
                ("a:not(.b)", true),
                ("a::after", true),
                ("a.b", true),
                ("a > b", true),
                ("a, b", true),
                ("#{50% - $n}", false),
                (".n-#{$n}", false),
                (":n-#{$n}", false),
                ("%foo", false),
                (".a { .b }", false),
                (".a { &:hover }", false),
                (".n-@{n}", false),
                (".a:extend(.a)", false),
                (".a:extend(.a all)", false),
                ("a { &:extend(.a all) }", false),
                (".foo()", false),
                (".foo(@a)", false),
                (".foo(@a: 5px)", false),
                ("a\n// comment\nb", false),
                ("a\n//comment\nb", false),
                ("<% COLORS.each do |color| %>\na", false),
                ("<% eng %>\na", false),
            ],
        );
    }

    #[test]
    #[ignore = "diverges from stylelint: Less mixin call followed by more selector parts is treated as standard"]
    fn is_standard_syntax_selector_less_mixin_divergence() {
        for s in [
            ".foo().bar",
            ".foo(@a, @b).bar",
            ".foo()#bar",
            ".foo()#bar",
            ".foo() bar",
            ".foo() + bar",
            ".foo() > bar",
            ".foo() ~ bar",
            ".foo()[bar]",
            ".foo()[bar='baz']",
        ] {
            assert!(!is_standard_syntax_selector(s), "{s:?}");
        }
    }

    #[test]
    fn is_standard_syntax_url_upstream() {
        check(
            is_standard_syntax_url,
            &[
                ("", true),
                ("some/path/to/file.png", true),
                ("'some/path/to/file.png'", true),
                ("\"some/path/to/file.png\"", true),
                ("/some/path/to/file.png", true),
                ("//www.domain.com/file.jpg", true),
                ("./some/path/to/file.png", true),
                ("../some/path/to/file.png", true),
                ("https://www.domain.com:8080/file.jpg", true),
                (
                    "data:image/gif;base64,R0lGODlhEAAQAMQAAORHHOVSKudfOulrSOp3WOyDZu6QdvCchPGolfO0o/XBs/fNwfjZ0frl3/zy7////wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAkAABAALAAAAAAQABAAAAVVICSOZGlCQAosJ6mu7fiyZeKqNKToQGDsM8hBADgUXoGAiqhSvp5QAnQKGIgUhwFUYLCVDFCrKUE1lBavAViFIDlTImbKC5Gm2hB0SlBCBMQiB0UjIQA7",
                    true,
                ),
                ("http://domain.com:8080/path/to$to/file$2x.ext?$1=$2", true),
                (
                    "'http://domain.com:8080/path$path/to$to/$file$2x.ext?$1$2=$1$2'",
                    true,
                ),
                (
                    "\"http://domain.com:8080/path$path/to$to/$file$2x.ext?$1$2=$1$2\"",
                    true,
                ),
                ("http://domain.com:8080/path/to@to/file@2x.ext?@1=@2", true),
                (
                    "'http://domain.com:8080/path$path/to@to/$file@2x.ext?@1$2=$1@2'",
                    true,
                ),
                (
                    "\"http://domain.com:8080/path$path/to@to/$file@2x.ext?@1$2=$1@2\"",
                    true,
                ),
                ("\"$url/path\"", true),
                ("\"some/$url/path\"", true),
                ("\"some/$url\"", true),
                ("\"@url/path\"", true),
                ("\"some/@url/path\"", true),
                ("\"some/@url\"", true),
                ("@url/path", true),
                ("some/@url/path", true),
                ("some/@url", true),
                ("@{less-variable}", false),
                ("@{less-variable}/path", false),
                ("some/@{less-variable}/path", false),
                ("some/@{less-variable}", false),
                ("#{$sass-variable}-color/images/bg.png", false),
                ("images/#{$sass-variable}-color/bg.png", false),
                ("images/#{$sass-variable}.png", false),
                ("'#{$sass-variable}-color/images/bg.png'", false),
                ("'images/#{$sass-variable}-color/bg.png'", false),
                ("'images/#{$sass-variable}'", false),
                ("\"#{$sass-variable}-color/images/bg.png\"", false),
                ("\"images/#{$sass-variable}-color/bg.png\"", false),
                ("\"images/#{$sass-variable}\"", false),
                ("$sass-variable", false),
                ("$sass-variable + 'foo'", false),
                ("$sass-variable + \"foo\"", false),
                ("$sass-variable+'foo'", false),
                ("$sass-variable + foo", false),
                ("$sass-variable + 'foo@bar'", false),
                ("$sass-variable + 'foo#bar'", false),
                ("$sass-variable + 'foo?bar'", false),
                ("$sass-variable + 'foo?bar#qux'", false),
                ("test + $sass-variable + foo", false),
                ("#{$sass-variable + foo}", false),
                ("\"#{$sass-variable + foo}\"", false),
                ("@less-variable", false),
                ("\"@{less-variable}\"", false),
                ("\"~@{less-variable}\"", false),
                ("'@{less-variable}-color/images/bg.png'", false),
                ("'images/@{less-variable}-color/bg.png'", false),
                ("'images/@{less-variable}'", false),
                ("\"@{less-variable}-color/images/bg.png\"", false),
                ("\"images/@{less-variable}-color/bg.png\"", false),
                ("\"images/@{less-variable}\"", false),
            ],
        );
    }

    #[test]
    fn is_numbery_upstream() {
        check(
            is_numbery,
            &[
                ("1", true),
                ("21 ", true),
                (" 212", true),
                ("232 ", true),
                ("", false),
                (" ", false),
                ("a", false),
                ("0", true),
                ("1", true),
                ("-1", true),
                ("0.1", true),
                ("-0.1", true),
                ("NaN", false),
            ],
        );
    }

    #[test]
    fn is_valid_hex_upstream() {
        check(
            is_valid_hex,
            &[
                ("#333", true),
                ("#a3b", true),
                ("#333a", true),
                ("#333afe", true),
                ("#333afeaa", true),
                ("a", false),
                ("aaa", false),
                ("$aaa", false),
                ("@aaa", false),
                ("var(aaa)", false),
                ("#z1", false),
                ("#00000", false),
                ("#000000000", false),
                ("#33z", false),
            ],
        );
    }

    #[test]
    fn is_valid_identifier_upstream() {
        check(
            is_valid_identifier,
            &[
                ("foo", true),
                ("foo1", true),
                ("1", false),
                ("-1", false),
                ("--foo", true),
                ("has a space", false),
                ("", false),
                (" ", false),
                ("foö", false),
                ("\\26 B", true),
                ("\\000026B", true),
                ("te'st", false),
                ("te\"st", false),
                ("te\\'st", true),
                ("te\\\"st", true),
                ("te\\'s\\'t", true),
                ("te\\\"s\\\"t", true),
            ],
        );
    }

    #[test]
    fn get_scheme_from_url_upstream() {
        let cases = [
            ("", None),
            (":", None),
            ("://", None),
            ("./file.jpg", None),
            ("/path/to/file.jpg", None),
            ("//www.example.com/file.jpg", None),
            ("http://www.example.com/file.jpg", Some("http")),
            ("HTTPS://www.example.com/file.jpg", Some("https")),
            (
                "data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs=",
                Some("data"),
            ),
        ];
        for (input, want) in cases {
            assert_eq!(get_scheme_from_url(input).as_deref(), want, "{input:?}");
        }
    }

    #[test]
    fn get_dimension_upstream() {
        let valid = [
            ("1.1s", "1.1", "s"),
            ("1.1000s", "1.1000", "s"),
            ("1.1S", "1.1", "S"),
            ("+1.1s", "+1.1", "s"),
            ("+1.1000s", "+1.1000", "s"),
            ("-1.1s", "-1.1", "s"),
            ("-1.1000s", "-1.1000", "s"),
            ("1.1e10s", "1.1e10", "s"),
            ("+1.1e10s", "+1.1e10", "s"),
            ("-1.1e10s", "-1.1e10", "s"),
            ("1.1e+10s", "1.1e+10", "s"),
            ("1.1e-10s", "1.1e-10", "s"),
            ("+1.1e+10s", "+1.1e+10", "s"),
            ("+1.1e-10s", "+1.1e-10", "s"),
            ("-1.1e-10s", "-1.1e-10", "s"),
            ("-1.1e+10s", "-1.1e+10", "s"),
            ("1px", "1", "px"),
            ("1pX", "1", "pX"),
            ("1PX", "1", "PX"),
            ("+1px", "+1", "px"),
            ("-1px", "-1", "px"),
            ("1e1px", "1e1", "px"),
            ("+1e1px", "+1e1", "px"),
            ("-1e1px", "-1e1", "px"),
            ("1e-1px", "1e-1", "px"),
            ("1e+1px", "1e+1", "px"),
            ("+1e+1px", "+1e+1", "px"),
            ("+1e-1px", "+1e-1", "px"),
            ("-1e-1px", "-1e-1", "px"),
            ("-1e+1px", "-1e+1", "px"),
            (".1s", ".1", "s"),
            ("+.1s", "+.1", "s"),
            ("-.1s", "-.1", "s"),
            ("+.1000s", "+.1000", "s"),
            ("-.1000s", "-.1000", "s"),
            (".1e1s", ".1e1", "s"),
            ("+.1e1s", "+.1e1", "s"),
            ("-.1e1s", "-.1e1", "s"),
            (".1e-1s", ".1e-1", "s"),
            (".1e+1s", ".1e+1", "s"),
            ("+.1e+1s", "+.1e+1", "s"),
            ("+.1e-1s", "+.1e-1", "s"),
            ("-.1e-1s", "-.1e-1", "s"),
            ("-.1e+1s", "-.1e+1", "s"),
            ("100%", "100", "%"),
            ("100", "100", ""),
            ("0\\0", "0", ""),
            ("10px\\9", "10", "px"),
            ("6e-2px", "6e-2", "px"),
            (".0", ".0", ""),
            ("+.0", "+.0", ""),
            ("-.0", "-.0", ""),
            (".0e1", ".0e1", ""),
            ("+.0e1", "+.0e1", ""),
            ("-.0e1", "-.0e1", ""),
        ];
        for (input, number, unit) in valid {
            assert_eq!(
                get_dimension(input),
                Some((number.to_string(), unit.to_string())),
                "{input:?}"
            );
        }
        let invalid = [
            "",
            "#fff",
            "#000",
            "#zzzzzz",
            "#F",
            "#PX",
            "\"100\"",
            "\"100px\"",
            " ",
            "/",
            "+",
            "word",
            "px",
            "url()",
            "url()px",
            "$variable",
            "${$variable}",
            "${$variable}px",
            "@variable",
            "e1",
            "wordPX",
            "..0",
        ];
        for input in invalid {
            assert_eq!(get_dimension(input), None, "{input:?}");
        }
    }

    #[test]
    fn url_arguments_ranges() {
        let cases: [(&str, Vec<(usize, usize)>); 11] = [
            ("", vec![]),
            ("url(a.png)", vec![(4, 9)]),
            ("URL( 'a)b' )", vec![(4, 11)]),
            ("url(\"a\\\")\")", vec![(4, 10)]),
            ("url(a) , url(b)", vec![(4, 5), (13, 14)]),
            ("image-set(url(a))", vec![(14, 15)]),
            ("url(a(b)c)", vec![(4, 9)]),
            ("url()", vec![(4, 4)]),
            ("myurl(a) -url(b) _url(c)", vec![]),
            ("url(a", vec![]),
            ("url", vec![]),
        ];
        for (input, want) in cases {
            assert_eq!(url_arguments(input), want, "{input:?}");
        }
    }

    #[test]
    fn media_feature_names_ranges() {
        let cases: [(&str, Vec<(usize, usize)>); 11] = [
            ("", vec![]),
            ("screen", vec![]),
            ("screen and (min-width: 100px)", vec![(12, 21)]),
            ("(color)", vec![(1, 6)]),
            ("(100px < width <= 900px)", vec![(9, 14)]),
            (
                "not all and (monochrome), (min-resolution: 2dppx)",
                vec![(13, 23), (27, 41)],
            ),
            ("(min-width: calc(1px + 2px))", vec![(1, 10)]),
            ("(width: 'a)b')", vec![(1, 6)]),
            ("(--custom: 1px) and ($var) and (@less)", vec![]),
            ("foo(bar)", vec![]),
            ("(width", vec![]),
        ];
        for (input, want) in cases {
            assert_eq!(media_feature_names(input), want, "{input:?}");
        }
    }

    #[test]
    fn media_feature_groups_ranges() {
        assert!(groups("").is_empty());
        assert!(groups("screen").is_empty());
        assert!(groups("(--foo: 1px)").is_empty());
        assert!(groups("(width").is_empty());
        assert_eq!(
            groups("(min-width: 100px)"),
            vec![(1, 10, vec![(12, 17, 15)], vec![(12, 17)])]
        );
        assert_eq!(
            groups("(orientation: landscape)"),
            vec![(1, 12, vec![], vec![(14, 23)])]
        );
        assert_eq!(
            groups("(400px <= width <= 700px)"),
            vec![(
                10,
                15,
                vec![(1, 6, 4), (19, 24, 22)],
                vec![(1, 6), (19, 24)]
            )]
        );
        assert_eq!(
            groups("(aspect-ratio: 16/9)"),
            vec![(1, 13, vec![], vec![(15, 17), (18, 19)])]
        );
        assert_eq!(
            groups("(min-width: 50%)"),
            vec![(1, 10, vec![], vec![(12, 15)])]
        );
        assert_eq!(
            groups("(min-resolution: 2dppx) and (color)"),
            vec![
                (1, 15, vec![(17, 22, 18)], vec![(17, 22)]),
                (29, 34, vec![], vec![])
            ]
        );
        assert_eq!(
            groups("(min-width: calc(1px + 2px))"),
            vec![(1, 10, vec![], vec![])]
        );
    }

    #[test]
    fn selector_attribute_blocks_ranges() {
        let cases: [(&str, Vec<(usize, usize)>); 9] = [
            ("", vec![]),
            ("a", vec![]),
            ("a[href]", vec![(2, 6)]),
            ("[a=\"]\"][b]", vec![(1, 6), (8, 9)]),
            ("[a='x\\'y']", vec![(1, 9)]),
            ("a > [b~=c]", vec![(5, 9)]),
            (":not([a]) [b]", vec![(11, 12)]),
            ("[a", vec![]),
            ("[]", vec![(1, 1)]),
        ];
        for (input, want) in cases {
            assert_eq!(selector_attribute_blocks(input), want, "{input:?}");
        }
    }

    #[test]
    fn selector_attribute_blocks_nested_ranges() {
        let cases: [(&str, Vec<(usize, usize)>); 7] = [
            ("", vec![]),
            ("a[href]", vec![(2, 6)]),
            (":not([a])", vec![(6, 7)]),
            (":is([a], [b=\"(]\"])", vec![(5, 6), (10, 16)]),
            ("\"[x]\"[y]", vec![(6, 7)]),
            ("[a='x\\'y']", vec![(1, 9)]),
            ("[a", vec![]),
        ];
        for (input, want) in cases {
            assert_eq!(selector_attribute_blocks_nested(input), want, "{input:?}");
        }
    }

    #[test]
    fn selector_combinators_ranges() {
        let cases: [(&str, Vec<Combo>); 14] = [
            ("", vec![]),
            ("a", vec![]),
            (" a ", vec![]),
            ("a b", vec![(1, 2, " ")]),
            ("a\n\tb", vec![(1, 3, " ")]),
            ("a > b", vec![(2, 3, ">")]),
            ("a+b~c", vec![(1, 2, "+"), (3, 4, "~")]),
            ("a || b", vec![(2, 4, "||")]),
            ("a /deep/ b", vec![]),
            ("a:not(b > c) d", vec![(12, 13, " ")]),
            ("[a~=b] c", vec![(6, 7, " ")]),
            ("[a=\"x > y\"]", vec![]),
            ("a/* > */b", vec![]),
            ("a:is(b, c)+d", vec![(10, 11, "+")]),
        ];
        for (input, want) in cases {
            assert_eq!(selector_combinators(input), combos(&want), "{input:?}");
        }
    }

    #[test]
    fn selector_combinators_nested_ranges() {
        let cases: [(&str, Vec<Combo>); 9] = [
            ("", vec![]),
            ("a > b", vec![(2, 3, ">")]),
            ("a:not(b > c)", vec![(8, 9, ">")]),
            ("a:not(b > c) d", vec![(8, 9, ">"), (12, 13, " ")]),
            (":is(a b)", vec![(5, 6, " ")]),
            ("[a='>'] + [b~=c]", vec![(8, 9, "+")]),
            ("a /deep/ b", vec![]),
            ("a/* ~ */b", vec![]),
            ("a || b", vec![(2, 4, "||")]),
        ];
        for (input, want) in cases {
            assert_eq!(
                selector_combinators_nested(input),
                combos(&want),
                "{input:?}"
            );
        }
    }

    #[test]
    fn selector_pseudos_ranges() {
        let cases: [(&str, Vec<Pseudo>); 11] = [
            ("", vec![]),
            ("a", vec![]),
            ("a:hover", vec![(1, 7, 2, 7, false)]),
            ("a::before", vec![(1, 9, 3, 9, true)]),
            ("a:not(:hover)", vec![(1, 5, 2, 5, false)]),
            (
                "a:nth-child(2n+1)::after",
                vec![(1, 11, 2, 11, false), (17, 24, 19, 24, true)],
            ),
            (":hov\\:er", vec![(0, 8, 1, 8, false)]),
            ("[href=':x']:focus", vec![(11, 17, 12, 17, false)]),
            ("a/* :hover */", vec![]),
            ("a:", vec![]),
            ("a::", vec![]),
        ];
        for (input, want) in cases {
            assert_eq!(selector_pseudos(input), want, "{input:?}");
        }
    }

    #[test]
    fn selector_pseudos_nested_ranges() {
        let cases: [(&str, Vec<Pseudo>); 7] = [
            ("", vec![]),
            ("a:hover", vec![(1, 7, 2, 7, false)]),
            (
                "a:not(:hover)",
                vec![(1, 5, 2, 5, false), (6, 12, 7, 12, false)],
            ),
            (
                ":is(a::before, [x=':y'])",
                vec![(0, 3, 1, 3, false), (5, 13, 7, 13, true)],
            ),
            (
                "a:not(:is(:focus))",
                vec![
                    (1, 5, 2, 5, false),
                    (6, 9, 7, 9, false),
                    (10, 16, 11, 16, false),
                ],
            ),
            ("a/* :hover */", vec![]),
            ("a::", vec![]),
        ];
        for (input, want) in cases {
            assert_eq!(selector_pseudos_nested(input), want, "{input:?}");
        }
    }

    #[test]
    fn count_selector_classes_cases() {
        let cases = [
            ("", 0),
            ("a", 0),
            (".a", 1),
            (".a.b", 2),
            ("a.b > .c", 2),
            (".-a ._b", 2),
            (":not(.a)", 1),
            (".1a", 0),
            ("a\\.b", 0),
            ("[class='.x']", 0),
            ("[a=b].c", 1),
            ("/* .a */", 0),
            ("\".a\"", 0),
        ];
        for (input, want) in cases {
            assert_eq!(count_selector_classes(input), want, "{input:?}");
        }
    }

    #[test]
    fn count_selector_universals_cases() {
        let cases = [
            ("", 0),
            ("a", 0),
            ("*", 1),
            ("*.a > *", 2),
            ("ns|*", 1),
            (":not(*)", 1),
            ("[a='*']", 0),
            ("[a*=b]", 0),
            ("/* * */a", 0),
            ("a\\*", 0),
        ];
        for (input, want) in cases {
            assert_eq!(count_selector_universals(input), want, "{input:?}");
        }
    }

    #[test]
    fn count_selector_types_cases() {
        let cases = [
            ("", 0),
            ("*", 0),
            (".a", 0),
            ("a", 1),
            ("a.b", 1),
            ("a#b", 1),
            ("a:hover", 1),
            ("a > b", 2),
            ("a + b ~ c", 3),
            ("a, b", 2),
            (":not(a)", 1),
            ("svg|circle", 1),
            ("[type=a]", 0),
            ("a[x='b c']", 1),
            ("a:nth-child(2n+1)", 1),
            ("a /* b */", 1),
        ];
        for (input, want) in cases {
            assert_eq!(count_selector_types(input), want, "{input:?}");
        }
    }

    #[test]
    #[ignore = "diverges from stylelint: descendant combinator and An+B keywords miscounted (selector-max-type)"]
    fn count_selector_types_divergence() {
        let cases = [
            ("a b", 2),
            ("a\tb", 2),
            (".foo:nth-child(odd)", 0),
            (".foo:nth-child(even)", 0),
            (".foo:nth-child(n)", 0),
            (".foo:nth-child(-n)", 0),
        ];
        for (input, want) in cases {
            assert_eq!(count_selector_types(input), want, "{input:?}");
        }
    }
}

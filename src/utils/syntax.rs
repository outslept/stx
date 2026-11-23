//! Lexical classification and standard CSS vs. preprocessor syntax validation.
//!
//! Provides predicates to distinguish standard W3C CSS constructs from SCSS, Sass, Less,
//! and CSS-in-JS extensions (such as variables, mixins, and interpolations), as well as
//! low-level validators for identifiers, hex colors, dimensions, and URLs.

/// Returns `true` if `s` contains Less variable interpolation (`@{...}`).
#[inline]
pub fn has_less_interpolation(s: &str) -> bool {
    has_delimited(s, b"@{", b'}')
}

/// Returns `true` if `s` contains SCSS/Sass interpolation (`#{...}`).
#[inline]
pub fn has_scss_interpolation(s: &str) -> bool {
    has_delimited(s, b"#{", b'}')
}

/// Returns `true` if `s` contains PostCSS simple-vars interpolation (`$(...)`).
#[inline]
pub fn has_psv_interpolation(s: &str) -> bool {
    has_delimited(s, b"$(", b')')
}

/// Returns `true` if `s` contains template literal or brace interpolation (`{...}`).
#[inline]
pub fn has_tpl_interpolation(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() {
                if bytes[j] == b'}' && j > i + 1 {
                    return true;
                }
                j += 1;
            }
        }
        i += 1;
    }
    false
}

/// Returns `true` if `s` contains any supported preprocessor or template interpolation.
pub fn has_interpolation(s: &str) -> bool {
    has_less_interpolation(s)
        || has_scss_interpolation(s)
        || has_psv_interpolation(s)
        || has_tpl_interpolation(s)
}

fn has_delimited(s: &str, open: &[u8; 2], close: u8) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 4 {
        return false;
    }
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == open[0] && bytes[i + 1] == open[1] {
            let mut j = i + 2;
            while j < bytes.len() {
                if bytes[j] == close && j > i + 2 {
                    return true;
                }
                j += 1;
            }
        }
        i += 1;
    }
    false
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

/// Returns `true` if `word` starts with an ASCII case-insensitive `var(` call.
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
        if let Some(end) = rest.find("__") {
            if end > 0 && !rest[..end].chars().any(char::is_whitespace) {
                return false;
            }
        }
    }
    if let Some(paren_idx) = value.find('(') {
        let before_paren = &value[..paren_idx];
        if let Some(dot_idx) = before_paren.rfind('.') {
            if dot_idx > 0 {
                let after_dot = &before_paren[dot_idx + 1..];
                let before_dot = &before_paren[..dot_idx];
                let before_char = before_dot.chars().next_back();
                if !after_dot.is_empty()
                    && after_dot.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && after_dot
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    && before_char
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    return false;
                }
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
    if let Some(open_idx) = trimmed.rfind("(@") {
        if trimmed.ends_with(')') && open_idx < trimmed.len() - 1 {
            return false;
        }
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
    if url.contains('$')
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
        })
    {
        return false;
    }
    true
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

/// Returns `true` if `ident` is a valid CSS level 2 identifier, accounting for ISO 10646
/// hex escapes and single-character backslash escapes.
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

/// Extracts the lowercase URI scheme from `url_str` if present and valid.
///
/// Returns `None` for protocol-relative URLs (`//example.com`), `host:port` pairs,
/// relative paths, or malformed URLs.
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
    let scheme = scheme_raw.to_ascii_lowercase();
    let rest = &url_str[colon_idx + 1..];
    if scheme != "data" && !rest.starts_with("//") {
        return None;
    }
    if matches!(scheme.as_str(), "http" | "https" | "ws" | "wss" | "ftp") {
        let after_slashes = &rest[2..];
        if after_slashes.is_empty()
            || after_slashes.starts_with('/')
            || after_slashes.starts_with(':')
        {
            return None;
        }
    }
    Some(scheme)
}

/// Splits a numeric word token into `(number_literal, unit_suffix)` after stripping
/// legacy IE backslash hacks (`\0`, `\9`).
///
/// Returns `None` for hex colors, non-standard syntax tokens, or words without a leading number.
pub fn get_dimension(word: &str) -> Option<(String, String)> {
    if word.is_empty() || word.starts_with('#') || !is_standard_syntax_value(word) {
        return None;
    }
    let blurred = blur_interpolation(word, "")
        .replace("\\0", "")
        .replace("\\9", "");
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

//! String manipulation, comment stripping, vendor prefix extraction, and normalization.

/// Extracts a leading CSS vendor prefix (e.g., `"-webkit-"`, `"-moz-"`, `"-ms-"`, `"-o-"`).
///
/// Returns `None` if the identifier is not vendor-prefixed or is a custom property (`--*`).
pub fn vendor_prefix(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.len() < 3 || bytes[0] != b'-' || bytes[1] == b'-' {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'-' {
            if i > 1 {
                return Some(&s[..=i]);
            }
            return None;
        }
        if !b.is_ascii_alphanumeric() && b != b'_' {
            return None;
        }
        i += 1;
    }
    None
}

/// Returns `s` with its leading vendor prefix removed, if present.
#[inline]
pub fn unprefixed(s: &str) -> &str {
    if let Some(prefix) = vendor_prefix(s) {
        &s[prefix.len()..]
    } else {
        s
    }
}

/// Replaces block comments (`/* ... */`) with ASCII spaces while preserving exact byte offsets
/// and newline positions.
///
/// Strings (`'...'` and `"..."`) are respected so comment-like sequences inside string literals
/// remain untouched.
pub fn strip_comments_preserving_offsets(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    let mut in_str: Option<u8> = None;
    while i < out.len() {
        let b = out[i];
        if let Some(q) = in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if b == b'\'' || b == b'"' {
            in_str = Some(b);
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < out.len() && out[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < out.len() && !(out[i] == b'*' && out[i + 1] == b'/') {
                i += 1;
            }
            let end = if i + 1 < out.len() {
                i += 2;
                i
            } else {
                out.len()
            };
            for ch in &mut out[start..end] {
                if *ch != b'\n' && *ch != b'\r' {
                    *ch = b' ';
                }
            }
            continue;
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

/// Removes block comments (`/* ... */`) from a CSS snippet while respecting string literals.
pub fn strip_comments(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_str: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_str {
            out.push(b);
            if b == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if b == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if b == b'\'' || b == b'"' {
            in_str = Some(b);
            out.push(b);
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(b);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

/// Splits `raw` by a top-level byte `delimiter`, ignoring occurrences nested inside parentheses,
/// brackets, braces, string literals, or block comments.
///
/// Each returned tuple contains `(start_offset, end_offset, slice)`.
pub fn split_top_level<'a>(raw: &'a str, delimiter: u8) -> Vec<(usize, usize, &'a str)> {
    let bytes = raw.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_str: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if b == b'\'' || b == b'"' {
            in_str = Some(b);
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ if b == delimiter && depth == 0 => {
                parts.push((start, i, &raw[start..i]));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push((start, bytes.len(), &raw[start..]));
    parts
}

/// Trims ASCII whitespace from `&raw[start..end]` and returns `(trimmed_start, trimmed_end, slice)`.
pub fn trim_subslice<'a>(raw: &'a str, start: usize, end: usize) -> (usize, usize, &'a str) {
    let bytes = raw.as_bytes();
    let mut s = start;
    let mut e = end;
    while s < e && bytes[s].is_ascii_whitespace() {
        s += 1;
    }
    while e > s && bytes[e - 1].is_ascii_whitespace() {
        e -= 1;
    }
    (s, e, &raw[s..e])
}

/// Formats a floating-point number to at most 4 decimal places with trailing zeros stripped.
pub fn format_clean_float(val: f64) -> String {
    if (val - val.round()).abs() < 1e-9 {
        return (val.round() as i64).to_string();
    }
    let mut s = format!("{val:.4}");
    let len = s.trim_end_matches('0').trim_end_matches('.').len();
    s.truncate(len);
    if s == "-0" {
        return "0".to_string();
    }
    s
}

/// Returns `true` if `content` is a linter configuration directive comment
/// (`stylelint-disable*`, `stylelint-enable*`, `stx-disable*`, `stx-enable*`).
pub fn is_configuration_comment(content: &str) -> bool {
    let trimmed = content.trim();
    for prefix in ["stylelint-", "stx-"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let cmd = rest.split_whitespace().next().unwrap_or("");
            if matches!(
                cmd,
                "disable" | "enable" | "disable-line" | "disable-next-line"
            ) {
                return true;
            }
        }
    }
    false
}

/// Returns `true` if `c` is a CSS whitespace character (`' '`, `'\n'`, `'\t'`, `'\r'`, `'\x0c'`).
#[inline]
pub fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\t' | '\r' | '\x0c')
}

/// Returns `true` if `input` contains only CSS whitespace characters.
#[inline]
pub fn is_only_whitespace(input: &str) -> bool {
    input.chars().all(is_whitespace)
}

/// Returns `true` if `input` contains no line feed (`\n`) or carriage return (`\r`) characters.
#[inline]
pub fn is_single_line_string(input: &str) -> bool {
    !input.contains(['\n', '\r'])
}

/// Normalizes a pseudo-class or pseudo-element name by stripping leading colons and converting
/// to ASCII lowercase.
#[inline]
pub fn normalize_pseudo_name(value: &str) -> String {
    value.trim_start_matches(':').to_ascii_lowercase()
}

/// Collapses contiguous runs of whitespace in a selector combinator into a single ASCII space.
pub fn normalize_combinator(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_ws = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            in_ws = false;
            out.push(ch);
        }
    }
    out
}

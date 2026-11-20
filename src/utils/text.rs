use oxc_css_parser::Span;

/// Checks if a string contains SCSS `#{...}`, Less `@{...}`, or CSS-in-JS/Less `${...}` / `$(...)` interpolation.
pub fn has_interpolation(s: &str) -> bool {
    s.contains("#{") || s.contains("@{") || s.contains("${") || s.contains("$(")
}

/// Extracts the vendor prefix (e.g. `"-webkit-"` from `"-webkit-transform"`), if present.
pub fn vendor_prefix(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'-') {
        return None;
    }
    // Custom properties `--foo` do not have a vendor prefix
    if bytes.get(1) == Some(&b'-') {
        return None;
    }
    let rest = &s[1..];
    let second_dash = rest.find('-')?;
    if second_dash == 0 {
        return None;
    }
    let prefix = &s[..=second_dash + 1];
    Some(prefix)
}

/// Returns the string with any leading vendor prefix (`-webkit-`, `-moz-`, `-ms-`, `-o-`, etc.) removed.
pub fn unprefixed(s: &str) -> &str {
    if let Some(prefix) = vendor_prefix(s) {
        &s[prefix.len()..]
    } else {
        s
    }
}

/// Replaces block comments `/* ... */` (outside strings) with spaces of the exact same byte length
/// so that byte offsets into the string remain 1-to-1 with the original slice.
pub fn strip_comments_preserving_offsets(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            let quote = b;
            out.push(b);
            i += 1;
            while i < len {
                out.push(bytes[i]);
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                    out.push(bytes[i]);
                    i += 1;
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
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            out.push(b' ');
            out.push(b' ');
            i += 2;
            while i < len {
                if bytes[i] == b'*' && i + 1 < len && bytes[i + 1] == b'/' {
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                    break;
                }
                out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            continue;
        }
        out.push(b);
        i += 1;
    }

    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

/// Strips block comments `/* ... */` (outside strings) completely.
pub fn strip_comments(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            let quote = b;
            out.push(b);
            i += 1;
            while i < len {
                out.push(bytes[i]);
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                    out.push(bytes[i]);
                    i += 1;
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
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i < len {
                if bytes[i] == b'*' && i + 1 < len && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(b);
        i += 1;
    }

    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

/// Splits `raw` by `delimiter` (e.g. `b','`) at top-level (outside `()`, `[]`, `{}`, strings, and `/*...*/`).
/// Returns `(start_offset, end_offset, slice)` for each segment.
pub fn split_top_level<'a>(raw: &'a str, delimiter: u8) -> Vec<(usize, usize, &'a str)> {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut i = 0usize;

    while i < len {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            let q = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(len);
                    continue;
                }
                if bytes[i] == q {
                    i += 1;
                    break;
                }
                i += 1;
            }
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
        match b {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            _ if b == delimiter && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                parts.push((start, i, &raw[start..i]));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }

    parts.push((start, len, &raw[start..len]));
    parts
}

/// Returns `(trimmed_start_offset, trimmed_end_offset, trimmed_slice)` within `raw[start..end]`.
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

/// Scans `source` for all `@<target_name>` at-rules and returns `(prelude_span, prelude_raw)`
/// even when the prelude contains invalid CSS syntax.
pub fn scan_at_rule_preludes<'a>(source: &'a str, target_name: &str) -> Vec<(Span, &'a str)> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0usize;

    while i < len {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            let q = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(len);
                    continue;
                }
                if bytes[i] == q {
                    i += 1;
                    break;
                }
                i += 1;
            }
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
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'@' {
            let name_start = i + 1;
            let mut name_end = name_start;
            while name_end < len
                && (bytes[name_end].is_ascii_alphanumeric()
                    || bytes[name_end] == b'-'
                    || bytes[name_end] == b'_')
            {
                name_end += 1;
            }
            let name = &source[name_start..name_end];
            if name.eq_ignore_ascii_case(target_name) {
                let prelude_scan_start = name_end;
                let mut j = prelude_scan_start;
                let mut paren_depth = 0usize;
                let mut bracket_depth = 0usize;
                while j < len {
                    let cj = bytes[j];
                    if cj == b'"' || cj == b'\'' {
                        let q = cj;
                        j += 1;
                        while j < len {
                            if bytes[j] == b'\\' {
                                j = (j + 2).min(len);
                                continue;
                            }
                            if bytes[j] == q {
                                j += 1;
                                break;
                            }
                            j += 1;
                        }
                        continue;
                    }
                    if cj == b'/' && j + 1 < len && bytes[j + 1] == b'*' {
                        j += 2;
                        while j + 1 < len && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                            j += 1;
                        }
                        j = (j + 2).min(len);
                        continue;
                    }
                    match cj {
                        b'(' => paren_depth += 1,
                        b')' => paren_depth = paren_depth.saturating_sub(1),
                        b'[' => bracket_depth += 1,
                        b']' => bracket_depth = bracket_depth.saturating_sub(1),
                        b'{' | b';' | b'}' if paren_depth == 0 && bracket_depth == 0 => break,
                        _ => {}
                    }
                    j += 1;
                }
                let (p_start, p_end, trimmed) = trim_subslice(source, prelude_scan_start, j);
                if p_start < p_end {
                    results.push((
                        Span {
                            start: p_start as u32,
                            end: p_end as u32,
                        },
                        trimmed,
                    ));
                }
                i = j;
                continue;
            }
            i = name_end.max(i + 1);
            continue;
        }
        i += 1;
    }

    results
}

/// Equivalent to Stylelint's `isConfigurationComment`.
/// Returns `true` if the comment content is a configuration comment such as
/// `stylelint-disable`, `stylelint-enable`, `stylelint-disable-line`,
/// `stylelint-disable-next-line`, or `stylelint-configure`.
pub fn is_configuration_comment(content: &str) -> bool {
    let trimmed = content.trim();
    let base = match trimmed.split_once("--") {
        Some((before, _)) => before.trim(),
        None => trimmed,
    };

    for prefix in ["stylelint-", "stx-"] {
        if let Some(after_prefix) = base.strip_prefix(prefix) {
            for command in [
                "disable-next-line",
                "disable-line",
                "disable",
                "enable",
                "configure",
            ] {
                if let Some(rest) = after_prefix.strip_prefix(command) {
                    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

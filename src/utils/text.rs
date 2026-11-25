//! String manipulation, comment stripping, vendor prefix extraction, and normalization.

/// Extracts a leading CSS vendor prefix (e.g., `"-webkit-"`, `"-moz-"`, `"-ms-"`, `"-o-"`).
///
/// Returns `None` if the identifier is not vendor-prefixed or is a custom property (`--*`).
const FONT_SIZE_KEYWORDS: &[&str] = &[
    "xx-small",
    "x-small",
    "small",
    "medium",
    "large",
    "x-large",
    "xx-large",
    "xxx-large",
    "larger",
    "smaller",
];
const FONT_SIZE_UNITS: &[&str] = &[
    "px", "em", "rem", "%", "pt", "pc", "in", "cm", "mm", "vh", "vw", "ch", "ex",
];

pub fn font_size_end(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let token = value[start..i]
            .split('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if FONT_SIZE_KEYWORDS.contains(&token.as_str())
            || FONT_SIZE_UNITS.iter().any(|unit| {
                token
                    .strip_suffix(unit)
                    .is_some_and(|number| number.parse::<f64>().is_ok())
            })
        {
            return Some(i);
        }
    }
    None
}

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
pub fn split_top_level(raw: &str, delimiter: u8) -> Vec<(usize, usize, &str)> {
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
pub fn trim_subslice(raw: &str, start: usize, end: usize) -> (usize, usize, &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

    const VALUE: &str = "-1px -1px 1px rgba(0, 0, 0, 0.2) inset";

    #[test]
    fn vendor_prefix_upstream() {
        assert_eq!(vendor_prefix("-moz-color"), Some("-moz-"));
        assert_eq!(vendor_prefix("color"), None);
        assert_eq!(vendor_prefix(VALUE), None);
    }

    #[test]
    fn vendor_prefix_edge_cases() {
        for (input, expected) in [
            ("-webkit-box-shadow", Some("-webkit-")),
            ("-ms-", Some("-ms-")),
            ("-o-x", Some("-o-")),
            ("-a_b-c", Some("-a_b-")),
            ("--custom-prop", None),
            ("--", None),
            ("-", None),
            ("", None),
            ("-moz", None),
            ("-é-x", None),
        ] {
            assert_eq!(vendor_prefix(input), expected, "{input:?}");
        }
    }

    #[test]
    fn unprefixed_upstream() {
        assert_eq!(unprefixed("-moz-color"), "color");
        assert_eq!(unprefixed("color"), "color");
        assert_eq!(unprefixed(VALUE), VALUE);
    }

    #[test]
    fn unprefixed_edge_cases() {
        assert_eq!(unprefixed("-webkit-box-shadow"), "box-shadow");
        assert_eq!(unprefixed("--custom"), "--custom");
        assert_eq!(unprefixed("-ms-"), "");
        assert_eq!(unprefixed(""), "");
    }

    #[test]
    fn strip_comments_preserving_offsets_cases() {
        for (input, expected) in [
            ("", ""),
            ("a b", "a b"),
            ("a/* x */b", "a       b"),
            ("a/*\n*/b", "a  \n  b"),
            ("a/*\r\n*/b", "a  \r\n  b"),
            ("a/* x", "a    "),
            ("/**/", "    "),
            ("'/* x */'", "'/* x */'"),
            ("\"a\\\"/*\"/*c*/", "\"a\\\"/*\"     "),
            ("é/*ü*/x", "é      x"),
            ("a /* 1 */ b /* 2 */", "a         b        "),
        ] {
            let out = strip_comments_preserving_offsets(input);
            assert_eq!(out, expected, "{input:?}");
            assert_eq!(out.len(), input.len(), "{input:?}");
        }
    }

    #[test]
    fn strip_comments_cases() {
        for (input, expected) in [
            ("", ""),
            ("a b", "a b"),
            ("a/* x */b", "ab"),
            ("a/*\n*/b", "ab"),
            ("a/* x", "a"),
            ("/**/", ""),
            ("a/*/b", "a"),
            ("'/* x */'", "'/* x */'"),
            ("\"a\\\"/*\"/*c*/", "\"a\\\"/*\""),
            ("'a\\", "'a\\"),
            ("é/*ü*/x", "éx"),
            ("a /* 1 */ b /* 2 */", "a  b "),
        ] {
            assert_eq!(strip_comments(input), expected, "{input:?}");
        }
    }

    #[test]
    fn split_top_level_cases() {
        assert_eq!(split_top_level("", b','), vec![(0, 0, "")]);
        assert_eq!(split_top_level("a", b','), vec![(0, 1, "a")]);
        assert_eq!(
            split_top_level("a,b,", b','),
            vec![(0, 1, "a"), (2, 3, "b"), (4, 4, "")]
        );
        assert_eq!(
            split_top_level("a, f(b, [c, {d, e}]), 'x,y', \"p,q\", /* m, n */ z", b','),
            vec![
                (0, 1, "a"),
                (2, 20, " f(b, [c, {d, e}])"),
                (21, 27, " 'x,y'"),
                (28, 34, " \"p,q\""),
                (35, 48, " /* m, n */ z"),
            ]
        );
        assert_eq!(
            split_top_level("'a\\',b',c", b','),
            vec![(0, 7, "'a\\',b'"), (8, 9, "c")]
        );
        assert_eq!(
            split_top_level("a,/* b, c", b','),
            vec![(0, 1, "a"), (2, 9, "/* b, c")]
        );
        assert_eq!(
            split_top_level("a,'b, c", b','),
            vec![(0, 1, "a"), (2, 7, "'b, c")]
        );
        assert_eq!(
            split_top_level("a) ,b", b','),
            vec![(0, 3, "a) "), (4, 5, "b")]
        );
        assert_eq!(split_top_level("é,ü", b','), vec![(0, 2, "é"), (3, 5, "ü")]);
        assert_eq!(
            split_top_level("a b(c d)", b' '),
            vec![(0, 1, "a"), (2, 8, "b(c d)")]
        );
    }

    #[test]
    fn trim_subslice_cases() {
        assert_eq!(trim_subslice("  ab  ", 0, 6), (2, 4, "ab"));
        assert_eq!(trim_subslice("x \t\nab\n y", 1, 8), (4, 6, "ab"));
        assert_eq!(trim_subslice("   ", 0, 3), (3, 3, ""));
        assert_eq!(trim_subslice("", 0, 0), (0, 0, ""));
        assert_eq!(trim_subslice("a b", 1, 1), (1, 1, ""));
        assert_eq!(trim_subslice(" é ", 0, 4), (1, 3, "é"));
    }

    #[test]
    fn format_clean_float_cases() {
        for (input, expected) in [
            (0.0, "0"),
            (50.0, "50"),
            (-3.0, "-3"),
            (0.5, "0.5"),
            (-2.5, "-2.5"),
            (33.333_333, "33.3333"),
            (1.234_56, "1.2346"),
            (0.1 + 0.2, "0.3"),
            (1e-10, "0"),
            (-0.000_01, "0"),
            (12.300_000_1, "12.3"),
        ] {
            assert_eq!(format_clean_float(input), expected, "{input}");
        }
    }

    #[test]
    fn is_configuration_comment_upstream() {
        for (input, expected) in [
            ("stylelint-disable", true),
            ("stylelint-disable-line", true),
            ("stylelint-disable-next-line", true),
            ("stylelint-enable", true),
            ("stylelint-", false),
            ("stylelint-disable-", false),
            ("stylelint-disable-lineee", false),
            ("stylelint-disable-next", false),
            ("stylelint-enable-", false),
            ("", false),
            (" ", false),
            ("\t", false),
            ("foo", false),
        ] {
            assert_eq!(is_configuration_comment(input), expected, "{input:?}");
        }
    }

    #[test]
    fn is_configuration_comment_extra() {
        for (input, expected) in [
            ("stylelint-disable ", true),
            ("stylelint-disable\t", true),
            ("stylelint-disable --", true),
            ("stylelint-disable color-named -- reason", true),
            ("stylelint-disable-next-line\nfoo", true),
            ("stx-disable", true),
            ("stx-enable", true),
            ("stx-disable-line", true),
            ("stx-disable-next-line", true),
            ("stx-", false),
            ("stx-disablee", false),
            ("Stylelint-disable", false),
            ("stylelint disable", false),
            ("eslint-disable", false),
        ] {
            assert_eq!(is_configuration_comment(input), expected, "{input:?}");
        }
    }

    #[test]
    fn is_whitespace_cases() {
        for c in [' ', '\n', '\t', '\r', '\x0c'] {
            assert!(is_whitespace(c), "{c:?}");
        }
        for c in ['a', '\x0b', '\u{a0}', '\u{2003}', '\0', '.'] {
            assert!(!is_whitespace(c), "{c:?}");
        }
    }

    #[test]
    fn is_only_whitespace_upstream() {
        assert!(is_only_whitespace("\r\n \t \n   "));
        assert!(!is_only_whitespace("   s"));
        assert!(!is_only_whitespace("s\t"));
        assert!(!is_only_whitespace("\n  .\t"));
    }

    #[test]
    fn is_only_whitespace_extra() {
        assert!(is_only_whitespace(""));
        assert!(is_only_whitespace("\x0c"));
        assert!(!is_only_whitespace("\u{a0}"));
    }

    #[test]
    fn is_single_line_string_upstream() {
        assert!(is_single_line_string("foo"));
        assert!(is_single_line_string("foo bar"));
        assert!(!is_single_line_string("foo\nbar"));
        assert!(!is_single_line_string("foo\rbar"));
        assert!(!is_single_line_string("foo\nbar"));
    }

    #[test]
    fn is_single_line_string_extra() {
        assert!(is_single_line_string(""));
        assert!(is_single_line_string("a\tb\x0c"));
        assert!(!is_single_line_string("\r\n"));
    }

    #[test]
    fn normalize_pseudo_name_cases() {
        for (input, expected) in [
            ("hover", "hover"),
            (":Hover", "hover"),
            ("::BEFORE", "before"),
            (":::x", "x"),
            ("-WEBKIT-scrollbar", "-webkit-scrollbar"),
            ("NTH-CHILD", "nth-child"),
            ("a:b", "a:b"),
            ("", ""),
            ("::", ""),
            (":É", "É"),
        ] {
            assert_eq!(normalize_pseudo_name(input), expected, "{input:?}");
        }
    }

    #[test]
    fn normalize_combinator_upstream() {
        for (input, expected) in [
            ("   ", " "),
            ("\n\t ", " "),
            (">\n", "> "),
            (">", ">"),
            ("+", "+"),
            ("~", "~"),
        ] {
            assert_eq!(normalize_combinator(input), expected, "{input:?}");
        }
    }

    #[test]
    fn normalize_combinator_extra() {
        for (input, expected) in [
            ("", ""),
            (" > ", " > "),
            ("\r\n~\x0c", " ~ "),
            ("|| ", "|| "),
            (" >>> ", " >>> "),
        ] {
            assert_eq!(normalize_combinator(input), expected, "{input:?}");
        }
    }
}

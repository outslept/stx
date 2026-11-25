//! Scans raw stylesheet text for at-rule preludes, value tokens, whitespace gaps,
//! declarations, and selector items without requiring a valid AST.
//!
//! Spans and offsets are zero-based UTF-8 byte positions in the original source;
//! span ends are exclusive. These are lexical scans, not CSS grammar validators.

use crate::utils::{
    reference::{LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, in_set_ignore_case},
    text::trim_subslice,
};
use oxc_css_parser::{Span, token::Comment};

#[inline]
fn skip_quoted_string(bytes: &[u8], mut i: usize, end: usize) -> usize {
    let quote = bytes[i];
    i += 1;
    while i < end {
        if bytes[i] == b'\\' {
            i = (i + 2).min(end);
            continue;
        }
        if bytes[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    end
}

#[inline]
fn skip_block_comment(bytes: &[u8], mut i: usize, end: usize) -> usize {
    i += 2;
    while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
        i += 1;
    }
    (i + 2).min(end)
}

fn skip_interpolation_brace(bytes: &[u8], open: usize, end: usize) -> Option<usize> {
    let mut i = open + 1;
    let mut depth = 1usize;
    let mut quote = 0u8;
    while i < end {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i = (i + 2).min(end);
                continue;
            }
            if b == quote {
                quote = 0;
            }
        } else if b == b'\\' {
            i = (i + 2).min(end);
            continue;
        } else if b == b'\'' || b == b'"' {
            quote = b;
        } else if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i + 1);
            }
        }
        i += 1;
    }
    None
}

/// Finds nonempty preludes for `target_name` (without `@`), ignoring ASCII case.
///
/// Returns `(span, raw_text)` pairs in source order, with surrounding ASCII
/// whitespace removed and internal comments retained. The scan stops a prelude
/// at a top-level `{`, `;`, `}`, or end of input, so even `@import url(foo.css`
/// yields `url(foo.css` when the target is `import`.
pub fn scan_at_rule_preludes<'a>(source: &'a str, target_name: &str) -> Vec<(Span, &'a str)> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0usize;
    while i < len {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            i = skip_quoted_string(bytes, i, len);
            continue;
        }
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i = skip_block_comment(bytes, i, len);
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
                        j = skip_quoted_string(bytes, j, len);
                        continue;
                    }
                    if cj == b'/' && j + 1 < len && bytes[j + 1] == b'*' {
                        j = skip_block_comment(bytes, j, len);
                        continue;
                    }
                    if cj == b'{'
                        && j > prelude_scan_start
                        && matches!(bytes[j - 1], b'#' | b'@')
                        && let Some(end) = skip_interpolation_brace(bytes, j, len)
                    {
                        j = end;
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

/// Lexical category assigned by [`tokenize_value`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueTokenKind {
    /// Identifier-like text, including names starting with `$` or `@`.
    Ident,
    /// Number without a unit or percent sign.
    Number,
    /// Number immediately followed by `%`.
    Percentage,
    /// Number immediately followed by an identifier-like unit.
    Dimension {
        /// Byte length of the numeric prefix, including any sign and exponent.
        num_len: usize,
    },
    /// Quoted string, including its opening quote and closing quote if present.
    String,
    /// Function name plus the opening `(`, not its arguments.
    Function,
    /// A `,` separator.
    Comma,
    /// A `/` outside a block comment or string.
    Slash,
    /// A `:` separator.
    Colon,
    /// A closing `)`.
    CloseParen,
    /// One otherwise unclassified character, such as a standalone `(`.
    Other,
}

/// A token borrowed from a raw declaration value or other CSS fragment.
#[derive(Debug, Clone)]
pub struct ValueToken<'a> {
    /// Lexical category of the token.
    pub kind: ValueTokenKind,
    /// Exact source text covered by `span`, without unescaping or normalization.
    pub raw: &'a str,
    /// Byte range of `raw` in the full source.
    pub span: Span,
}

/// Tokenizes the supplied value span in source order, skipping ASCII whitespace
/// and block comments. Function arguments are separate tokens in the same list.
///
/// The end is clamped to `source.len()`; an empty or reversed range yields no
/// tokens. Use UTF-8 character boundaries for the effective range.
///
/// ```
/// use oxc_css_parser::Span;
/// use stx::utils::scan::tokenize_value;
///
/// let source = "x: var(--x)";
/// let tokens = tokenize_value(source, Span { start: 3, end: 11 });
/// let raw: Vec<_> = tokens.iter().map(|token| token.raw).collect();
/// assert_eq!(raw, ["var(", "--x", ")"]);
/// assert_eq!(tokens[0].span.start, 3);
/// ```
pub fn tokenize_value<'a>(source: &'a str, span: Span) -> Vec<ValueToken<'a>> {
    let start_idx = span.start as usize;
    let end_idx = (span.end as usize).min(source.len());
    if start_idx >= end_idx {
        return Vec::new();
    }
    let bytes = source.as_bytes();
    let mut i = start_idx;
    let mut tokens = Vec::new();
    while i < end_idx {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < end_idx && bytes[i + 1] == b'*' {
            i = skip_block_comment(bytes, i, end_idx);
            continue;
        }
        if b == b'"' || b == b'\'' {
            let tok_start = i;
            i = skip_quoted_string(bytes, i, end_idx);
            tokens.push(ValueToken {
                kind: ValueTokenKind::String,
                raw: &source[tok_start..i],
                span: Span {
                    start: tok_start as u32,
                    end: i as u32,
                },
            });
            continue;
        }
        if let Some(kind) = match b {
            b',' => Some(ValueTokenKind::Comma),
            b'/' => Some(ValueTokenKind::Slash),
            b':' => Some(ValueTokenKind::Colon),
            b')' => Some(ValueTokenKind::CloseParen),
            _ => None,
        } {
            tokens.push(ValueToken {
                kind,
                raw: &source[i..i + 1],
                span: Span {
                    start: i as u32,
                    end: (i + 1) as u32,
                },
            });
            i += 1;
            continue;
        }
        if is_number_start(bytes, i, end_idx) {
            let tok_start = i;
            if bytes[i] == b'+' || bytes[i] == b'-' {
                i += 1;
            }
            while i < end_idx && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < end_idx && bytes[i] == b'.' && i + 1 < end_idx && bytes[i + 1].is_ascii_digit() {
                i += 1;
                while i < end_idx && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < end_idx && (bytes[i] == b'e' || bytes[i] == b'E') {
                let mut exp_i = i + 1;
                if exp_i < end_idx && (bytes[exp_i] == b'+' || bytes[exp_i] == b'-') {
                    exp_i += 1;
                }
                if exp_i < end_idx && bytes[exp_i].is_ascii_digit() {
                    i = exp_i;
                    while i < end_idx && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let num_end = i;
            let num_len = num_end - tok_start;
            let kind = if i < end_idx && bytes[i] == b'%' {
                i += 1;
                ValueTokenKind::Percentage
            } else if i < end_idx && is_ident_start_byte(bytes[i]) {
                while i < end_idx && is_ident_char_byte(bytes[i]) {
                    i += 1;
                }
                ValueTokenKind::Dimension { num_len }
            } else {
                ValueTokenKind::Number
            };
            tokens.push(ValueToken {
                kind,
                raw: &source[tok_start..i],
                span: Span {
                    start: tok_start as u32,
                    end: i as u32,
                },
            });
            continue;
        }
        if is_ident_start_byte(b) || b == b'-' || b == b'$' || b == b'@' {
            let tok_start = i;
            i += 1;
            while i < end_idx && is_ident_char_byte(bytes[i]) {
                i += 1;
            }
            let kind = if i < end_idx && bytes[i] == b'(' {
                i += 1;
                ValueTokenKind::Function
            } else {
                ValueTokenKind::Ident
            };
            tokens.push(ValueToken {
                kind,
                raw: &source[tok_start..i],
                span: Span {
                    start: tok_start as u32,
                    end: i as u32,
                },
            });
            continue;
        }
        let ch_len = source[i..end_idx]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        tokens.push(ValueToken {
            kind: ValueTokenKind::Other,
            raw: &source[i..i + ch_len],
            span: Span {
                start: i as u32,
                end: (i + ch_len) as u32,
            },
        });
        i += ch_len;
    }
    tokens
}

fn is_number_start(bytes: &[u8], i: usize, end: usize) -> bool {
    let b = bytes[i];
    if b.is_ascii_digit() {
        return true;
    }
    if b == b'.' && i + 1 < end && bytes[i + 1].is_ascii_digit() {
        return true;
    }
    if (b == b'+' || b == b'-') && i + 1 < end {
        let next = bytes[i + 1];
        if next.is_ascii_digit() {
            return true;
        }
        if next == b'.' && i + 2 < end && bytes[i + 2].is_ascii_digit() {
            return true;
        }
    }
    false
}

fn is_ident_start_byte(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b >= 0x80
}

fn is_ident_char_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80
}

/// Whitespace and lexical context immediately before a node, from [`analyze_preceding_gap`].
#[derive(Debug, Clone)]
pub struct PreceedingGapInfo {
    /// No bytes other than ASCII whitespace precede the node.
    pub is_first_in_file: bool,
    /// The last non-whitespace byte before the node is `{`.
    pub is_first_nested: bool,
    /// A supplied comment ends at `gap_start` and does not follow a `}` after trimming whitespace.
    pub after_comment: bool,
    /// Number of LF bytes in the gap; CRLF counts once and a lone CR does not count.
    pub newline_count: usize,
    /// Leading word of the preceding semicolon-terminated statement, if found (e.g. `color`
    /// or `@import`); otherwise empty.
    pub prev_statement_head: String,
    /// The last non-whitespace byte before the node is `}`.
    pub prev_was_block: bool,
    /// Byte offset where the final run of spaces and tabs starts.
    pub indent_start: u32,
    /// Byte offset where the full run of ASCII whitespace starts.
    pub gap_start: u32,
    /// Spaces and tabs immediately before the node, possibly empty even when there is a gap.
    pub indent_slice: String,
}

/// Scans backward from `node_start` to describe the whitespace gap and preceding text.
///
/// `node_start` is a byte offset on a UTF-8 character boundary, clamped to the end
/// of `source`. Supply comments from the same source sorted by ascending span end.
/// With `"a {}\r\n\r\nb {}"` at offset `8`, the gap starts at `4`, the indent
/// starts at `8`, and the newline count is `2`.
pub fn analyze_preceding_gap(
    source: &str,
    comments: &[Comment<'_>],
    node_start: u32,
) -> PreceedingGapInfo {
    let start = (node_start as usize).min(source.len());
    let bytes = source.as_bytes();
    let mut indent_start = start;
    while indent_start > 0 && (bytes[indent_start - 1] == b' ' || bytes[indent_start - 1] == b'\t')
    {
        indent_start -= 1;
    }
    let indent_slice = source[indent_start..start].to_string();
    let mut gap_start = start;
    let mut newline_count = 0usize;
    while gap_start > 0 && bytes[gap_start - 1].is_ascii_whitespace() {
        gap_start -= 1;
        if bytes[gap_start] == b'\n' {
            newline_count += 1;
        }
    }
    if gap_start == 0 {
        return PreceedingGapInfo {
            is_first_in_file: true,
            is_first_nested: false,
            after_comment: false,
            newline_count,
            prev_statement_head: String::new(),
            prev_was_block: false,
            indent_start: indent_start as u32,
            gap_start: 0,
            indent_slice,
        };
    }
    let gap_start_u32 = gap_start as u32;
    let idx = comments.partition_point(|c| c.span.end < gap_start_u32);
    let after_comment = comments.get(idx).is_some_and(|c| {
        if c.span.end != gap_start_u32 {
            return false;
        }
        let before = source.get(..c.span.start as usize).unwrap_or("").trim_end();
        let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
        before.as_bytes().last() != Some(&b'}') || line_start == before.len()
    });
    let last_char = bytes[gap_start - 1];
    let is_first_nested = last_char == b'{';
    let prev_was_block = last_char == b'}';
    let mut prev_statement_head = String::new();
    if last_char == b';' {
        let mut scan = gap_start - 1;
        let mut paren_depth = 0usize;
        while scan > 0 {
            let b = bytes[scan - 1];
            if b == b')' {
                paren_depth += 1;
            } else if b == b'(' {
                paren_depth = paren_depth.saturating_sub(1);
            } else if paren_depth == 0 && (b == b';' || b == b'{' || b == b'}') {
                break;
            }
            scan -= 1;
        }
        let stmt_slice = source[scan..gap_start - 1].trim();
        prev_statement_head = stmt_slice
            .split(|c: char| c.is_ascii_whitespace() || c == ':' || c == '(')
            .next()
            .unwrap_or("")
            .to_string();
    }
    PreceedingGapInfo {
        is_first_in_file: false,
        is_first_nested,
        after_comment,
        newline_count,
        prev_statement_head,
        prev_was_block,
        indent_start: indent_start as u32,
        gap_start: gap_start_u32,
        indent_slice,
    }
}

/// A declaration-like statement found by [`scan_direct_declarations`].
#[derive(Debug, Clone)]
pub struct ScannedDecl<'a> {
    /// Trimmed property text before the first colon, as written in the source.
    pub prop: &'a str,
    /// Whether the trimmed value ends with `!important` (ASCII case-insensitive).
    /// Whitespace between `!` and `important` is not recognized.
    pub important: bool,
    /// Byte range of the trimmed declaration, including its semicolon if present
    /// but excluding leading block comments.
    pub span: Span,
}

/// Finds declaration-like statements directly inside a brace-delimited block.
///
/// `block_span` includes both braces; its interior must lie on UTF-8 character
/// boundaries. Returns matches in source order, skipping nested blocks and
/// statements beginning with `@`. The final declaration can omit its semicolon.
pub fn scan_direct_declarations<'a>(source: &'a str, block_span: Span) -> Vec<ScannedDecl<'a>> {
    let start = (block_span.start as usize + 1).min(source.len());
    let end = (block_span.end as usize)
        .saturating_sub(1)
        .min(source.len());
    if start >= end {
        return Vec::new();
    }
    let bytes = source.as_bytes();
    let mut i = start;
    let mut stmt_start = start;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut decls = Vec::new();
    while i < end {
        let b = bytes[i];
        if b == b'/' && i + 1 < end && bytes[i + 1] == b'*' {
            i = skip_block_comment(bytes, i, end);
            continue;
        }
        if b == b'"' || b == b'\'' {
            i = skip_quoted_string(bytes, i, end);
            continue;
        }
        match b {
            b'(' => {
                paren_depth += 1;
                i += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                i += 1;
            }
            b'{' if paren_depth == 0 => {
                brace_depth += 1;
                i += 1;
            }
            b'}' if paren_depth == 0 => {
                brace_depth = brace_depth.saturating_sub(1);
                i += 1;
                if brace_depth == 0 {
                    stmt_start = i;
                }
            }
            b';' if brace_depth == 0 && paren_depth == 0 => {
                if let Some(d) = parse_scanned_decl(source, stmt_start, i + 1) {
                    decls.push(d);
                }
                i += 1;
                stmt_start = i;
            }
            _ => i += 1,
        }
    }
    if brace_depth == 0
        && paren_depth == 0
        && stmt_start < end
        && let Some(d) = parse_scanned_decl(source, stmt_start, end)
    {
        decls.push(d);
    }
    decls
}

fn parse_scanned_decl<'a>(source: &'a str, start: usize, end: usize) -> Option<ScannedDecl<'a>> {
    let (mut decl_start, mut decl_end, mut trimmed) = trim_subslice(source, start, end);
    loop {
        if trimmed.is_empty() {
            return None;
        }
        if !trimmed.starts_with("/*") {
            break;
        }
        let after_comment = skip_block_comment(source.as_bytes(), decl_start, decl_end);
        (decl_start, decl_end, trimmed) = trim_subslice(source, after_comment, decl_end);
    }
    if trimmed.starts_with('@') {
        return None;
    }
    let colon_pos = trimmed.find(':')?;
    let prop = trimmed[..colon_pos].trim();
    if prop.is_empty()
        || prop.contains(char::is_whitespace)
        || prop.contains('{')
        || prop.contains('}')
    {
        return None;
    }
    let val_part = trimmed[colon_pos + 1..].trim_end_matches(';').trim();
    let important = val_part.len() >= 10
        && val_part.as_bytes()[val_part.len() - 10..].eq_ignore_ascii_case(b"!important");
    Some(ScannedDecl {
        prop,
        important,
        span: Span {
            start: decl_start as u32,
            end: decl_end as u32,
        },
    })
}

/// Lexical category of an item found by [`scan_selector_items`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorItemKind {
    /// A bare identifier in the selector text.
    Type,
    /// A class name prefixed with `.`.
    Class,
    /// An ID name prefixed with `#`.
    Id,
    /// A single-colon pseudo that is not a recognized legacy pseudo-element.
    PseudoClass {
        /// Byte range of the leading colon alone.
        colon_span: Span,
    },
    /// A double-colon pseudo or a recognized legacy form such as `:before`.
    PseudoElement {
        /// Byte range of the leading colon(s) alone.
        colon_span: Span,
        /// Whether the pseudo-element was written with one colon.
        single_colon: bool,
    },
    /// A bracketed attribute selector, possibly missing its closing bracket.
    Attribute {
        /// Byte range of the attribute contents, excluding brackets.
        inner_span: Span,
    },
}

/// An item and its source range from a raw selector list.
#[derive(Debug, Clone)]
pub struct ScannedSelectorItem<'a> {
    /// Item category and any delimiter spans.
    pub kind: SelectorItemKind,
    /// Raw name without a `.`, `#`, or colon prefix; for an attribute, the entire
    /// untrimmed text inside the brackets.
    pub name: &'a str,
    /// Byte range including the prefix or brackets, but excluding pseudo arguments.
    pub span: Span,
}

/// Reads selector items in `list_span` in source order without building a selector tree.
///
/// The end is clamped to `source.len()`; an empty or reversed range yields no
/// items. Use UTF-8 character boundaries for the effective range. Combinators
/// are not emitted, and selector items inside parentheses are included in the
/// same list: `a:not(.b, #c)` yields the names `a`, `not`, `b`, and `c`.
pub fn scan_selector_items<'a>(source: &'a str, list_span: Span) -> Vec<ScannedSelectorItem<'a>> {
    let start = list_span.start as usize;
    let end = (list_span.end as usize).min(source.len());
    if start >= end {
        return Vec::new();
    }
    let bytes = source.as_bytes();
    let mut i = start;
    let mut items = Vec::new();
    while i < end {
        let b = bytes[i];
        if b.is_ascii_whitespace()
            || matches!(
                b,
                b'>' | b'+' | b'~' | b',' | b'(' | b')' | b'*' | b'&' | b'|'
            )
        {
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < end && bytes[i + 1] == b'*' {
            i = skip_block_comment(bytes, i, end);
            continue;
        }
        if b == b'"' || b == b'\'' {
            i = skip_quoted_string(bytes, i, end);
            continue;
        }
        if b == b'[' {
            let attr_start = i;
            i += 1;
            let inner_start = i;
            let mut in_quote: Option<u8> = None;
            while i < end {
                if let Some(q) = in_quote {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(end);
                        continue;
                    }
                    if bytes[i] == q {
                        in_quote = None;
                    }
                    i += 1;
                    continue;
                }
                if bytes[i] == b'\\' {
                    i = (i + 2).min(end);
                    continue;
                }
                if bytes[i] == b'"' || bytes[i] == b'\'' {
                    in_quote = Some(bytes[i]);
                    i += 1;
                    continue;
                }
                if bytes[i] == b']' {
                    break;
                }
                i += 1;
            }
            let inner_end = i;
            if i < end && bytes[i] == b']' {
                i += 1;
            }
            items.push(ScannedSelectorItem {
                kind: SelectorItemKind::Attribute {
                    inner_span: Span {
                        start: inner_start as u32,
                        end: inner_end as u32,
                    },
                },
                name: &source[inner_start..inner_end],
                span: Span {
                    start: attr_start as u32,
                    end: i as u32,
                },
            });
            continue;
        }
        if b == b'.' || b == b'#' {
            let prefix = b;
            let tok_start = i;
            i += 1;
            let name_start = i;
            while i < end && is_ident_char_byte(bytes[i]) {
                i += 1;
            }
            if i > name_start {
                items.push(ScannedSelectorItem {
                    kind: if prefix == b'.' {
                        SelectorItemKind::Class
                    } else {
                        SelectorItemKind::Id
                    },
                    name: &source[name_start..i],
                    span: Span {
                        start: tok_start as u32,
                        end: i as u32,
                    },
                });
            }
            continue;
        }
        if b == b':' {
            let colon_start = i;
            i += 1;
            let is_double = i < end && bytes[i] == b':';
            if is_double {
                i += 1;
            }
            let colon_end = i;
            let name_start = i;
            while i < end && is_ident_char_byte(bytes[i]) {
                i += 1;
            }
            if i > name_start {
                let name = &source[name_start..i];
                let colon_span = Span {
                    start: colon_start as u32,
                    end: colon_end as u32,
                };
                let full_span = Span {
                    start: colon_start as u32,
                    end: i as u32,
                };
                if is_double || in_set_ignore_case(LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, name) {
                    items.push(ScannedSelectorItem {
                        kind: SelectorItemKind::PseudoElement {
                            colon_span,
                            single_colon: !is_double,
                        },
                        name,
                        span: full_span,
                    });
                } else {
                    items.push(ScannedSelectorItem {
                        kind: SelectorItemKind::PseudoClass { colon_span },
                        name,
                        span: full_span,
                    });
                }
            }
            continue;
        }
        if is_ident_start_byte(b) || b == b'-' {
            let tok_start = i;
            i += 1;
            while i < end && is_ident_char_byte(bytes[i]) {
                i += 1;
            }
            let name = &source[tok_start..i];
            items.push(ScannedSelectorItem {
                kind: SelectorItemKind::Type,
                name,
                span: Span {
                    start: tok_start as u32,
                    end: i as u32,
                },
            });
            continue;
        }
        i += 1;
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_css_parser::token::CommentKind;

    fn sp(start: u32, end: u32) -> Span {
        Span { start, end }
    }

    type PreludeCase<'a> = (&'a str, &'a str, &'a [(u32, u32, &'a str)]);
    type DeclCase<'a> = (&'a str, &'a [(&'a str, bool, &'a str)]);
    type ItemCase<'a> = (&'a str, Vec<(SelectorItemKind, &'a str, &'a str)>);

    fn whole(s: &str) -> Span {
        sp(0, s.len() as u32)
    }

    fn preludes<'a>(src: &'a str, name: &str) -> Vec<&'a str> {
        scan_at_rule_preludes(src, name)
            .into_iter()
            .map(|(span, raw)| {
                assert_eq!(span.slice(src), raw);
                raw
            })
            .collect()
    }

    #[test]
    fn scan_at_rule_preludes_offsets() {
        let cases: &[PreludeCase] = &[
            ("@media screen { a {} }", "media", &[(7, 13, "screen")]),
            ("@MEDIA print;", "media", &[(7, 12, "print")]),
            ("@media print", "media", &[(7, 12, "print")]),
            (
                "@import 'a.css';\n@import url(b.css) screen;",
                "import",
                &[(8, 15, "'a.css'"), (25, 42, "url(b.css) screen")],
            ),
            ("@media\r\n  print\r\n{}", "media", &[(10, 15, "print")]),
            ("@media-foo x; @media y;", "media", &[(21, 22, "y")]),
            (
                "@font-feature-values Fönt {}",
                "font-feature-values",
                &[(21, 26, "Fönt")],
            ),
            (
                "@media(min-width:1px){}",
                "media",
                &[(6, 21, "(min-width:1px)")],
            ),
        ];
        for (src, name, expected) in cases {
            let got: Vec<_> = scan_at_rule_preludes(src, name)
                .into_iter()
                .map(|(s, raw)| (s.start, s.end, raw))
                .collect();
            assert_eq!(&got, expected, "{src:?}");
        }
    }

    #[test]
    fn scan_at_rule_preludes_skips_and_nesting() {
        let cases: &[(&str, &str, &[&str])] = &[
            ("", "media", &[]),
            ("@media {}", "media", &[]),
            ("@media;", "media", &[]),
            ("@media   ;", "media", &[]),
            ("a {}", "media", &[]),
            ("a { content: '@media x'; } @media y {}", "media", &["y"]),
            (
                "a { content: \"\\\"@media x\"; } @media y {}",
                "media",
                &["y"],
            ),
            ("/* @media x; */ @media y {}", "media", &["y"]),
            ("// @media x\n@media y {}", "media", &["y"]),
            ("@media (a;b) {}", "media", &["(a;b)"]),
            ("@container [x;y] {}", "container", &["[x;y]"]),
            (
                "@supports (content: '{;}') and (a: b) {}",
                "supports",
                &["(content: '{;}') and (a: b)"],
            ),
            ("@media a /* { ; } */ b {}", "media", &["a /* { ; } */ b"]),
            ("@import url(foo.css", "import", &["url(foo.css"]),
            (
                "@media a {} @media b {} @supports c {}",
                "media",
                &["a", "b"],
            ),
            ("a { @media x { b {} } }", "media", &["x"]),
            ("@charset \"UTF-8\";", "charset", &["\"UTF-8\""]),
        ];
        for (src, name, expected) in cases {
            assert_eq!(&preludes(src, name), expected, "{src:?}");
        }
    }

    #[test]
    fn scan_at_rule_preludes_get_at_rule_params() {
        assert_eq!(preludes("@media all {}", "media"), ["all"]);
        assert_eq!(
            preludes("@media all /* foo */ and print {}", "media"),
            ["all /* foo */ and print"]
        );
    }

    #[test]
    fn scan_at_rule_preludes_interpolation() {
        assert_eq!(preludes("@media #{$q} {}", "media"), ["#{$q}"]);
        assert_eq!(preludes("@media @{q} {}", "media"), ["@{q}"]);
        assert_eq!(
            preludes("@media #{map-get($q, x)} {}", "media"),
            ["#{map-get($q, x)}"]
        );
    }

    fn tokens(src: &str, span: Span) -> Vec<(ValueTokenKind, &str)> {
        tokenize_value(src, span)
            .into_iter()
            .map(|t| {
                assert_eq!(t.span.slice(src), t.raw);
                (t.kind, t.raw)
            })
            .collect()
    }

    #[test]
    fn tokenize_value_kinds() {
        use ValueTokenKind::*;
        let dim = |n| Dimension { num_len: n };
        let cases: Vec<(&str, Vec<(ValueTokenKind, &str)>)> = vec![
            ("", vec![]),
            (" \t\r\n ", vec![]),
            (
                "1px solid red",
                vec![(dim(1), "1px"), (Ident, "solid"), (Ident, "red")],
            ),
            (
                "10% .5em -2 +3.5e-2px 1e3 1e 1E+2",
                vec![
                    (Percentage, "10%"),
                    (dim(2), ".5em"),
                    (Number, "-2"),
                    (dim(7), "+3.5e-2px"),
                    (Number, "1e3"),
                    (dim(1), "1e"),
                    (Number, "1E+2"),
                ],
            ),
            (
                "1.foo -.5 +.5%",
                vec![
                    (Number, "1"),
                    (Other, "."),
                    (Ident, "foo"),
                    (Number, "-.5"),
                    (Percentage, "+.5%"),
                ],
            ),
            (
                "rgba(0, 0, 0, .5)",
                vec![
                    (Function, "rgba("),
                    (Number, "0"),
                    (Comma, ","),
                    (Number, "0"),
                    (Comma, ","),
                    (Number, "0"),
                    (Comma, ","),
                    (Number, ".5"),
                    (CloseParen, ")"),
                ],
            ),
            (
                "url(a.png)",
                vec![
                    (Function, "url("),
                    (Ident, "a"),
                    (Other, "."),
                    (Ident, "png"),
                    (CloseParen, ")"),
                ],
            ),
            (
                "url( 'a b.png' )",
                vec![(Function, "url("), (String, "'a b.png'"), (CloseParen, ")")],
            ),
            (
                "calc((1px + 2px) * 3)",
                vec![
                    (Function, "calc("),
                    (Other, "("),
                    (dim(1), "1px"),
                    (Other, "+"),
                    (dim(1), "2px"),
                    (CloseParen, ")"),
                    (Other, "*"),
                    (Number, "3"),
                    (CloseParen, ")"),
                ],
            ),
            (
                "var(--x, fallback)",
                vec![
                    (Function, "var("),
                    (Ident, "--x"),
                    (Comma, ","),
                    (Ident, "fallback"),
                    (CloseParen, ")"),
                ],
            ),
            (
                "'a\\'b' \"c\\\"d\" 'x\"y'",
                vec![
                    (String, "'a\\'b'"),
                    (String, "\"c\\\"d\""),
                    (String, "'x\"y'"),
                ],
            ),
            ("'abc", vec![(String, "'abc")]),
            ("a /* b, c */ d /* open", vec![(Ident, "a"), (Ident, "d")]),
            (
                "1/2 a:b",
                vec![
                    (Number, "1"),
                    (Slash, "/"),
                    (Number, "2"),
                    (Ident, "a"),
                    (Colon, ":"),
                    (Ident, "b"),
                ],
            ),
            (
                "red !important",
                vec![(Ident, "red"), (Other, "!"), (Ident, "important")],
            ),
            (
                "$var @var -webkit-box --custom _u a - b",
                vec![
                    (Ident, "$var"),
                    (Ident, "@var"),
                    (Ident, "-webkit-box"),
                    (Ident, "--custom"),
                    (Ident, "_u"),
                    (Ident, "a"),
                    (Ident, "-"),
                    (Ident, "b"),
                ],
            ),
            (
                "#{$a} @{b}",
                vec![
                    (Other, "#"),
                    (Other, "{"),
                    (Ident, "$a"),
                    (Other, "}"),
                    (Ident, "@"),
                    (Other, "{"),
                    (Ident, "b"),
                    (Other, "}"),
                ],
            ),
            (
                "#fff #0f0",
                vec![(Other, "#"), (Ident, "fff"), (Other, "#"), (dim(1), "0f0")],
            ),
            (
                "Ünïcode 1€ 'ü'",
                vec![(Ident, "Ünïcode"), (dim(1), "1€"), (String, "'ü'")],
            ),
        ];
        for (src, expected) in cases {
            assert_eq!(tokens(src, whole(src)), expected, "{src:?}");
        }
    }

    #[test]
    fn tokenize_value_spans() {
        let src = "1px solid red";
        let got: Vec<_> = tokenize_value(src, whole(src))
            .iter()
            .map(|t| (t.span.start, t.span.end))
            .collect();
        assert_eq!(got, [(0, 3), (4, 9), (10, 13)]);
        assert_eq!(tokens(")", whole(")")), [(ValueTokenKind::CloseParen, ")")]);

        let src = "a\r\nb";
        let got: Vec<_> = tokenize_value(src, whole(src))
            .iter()
            .map(|t| (t.span.start, t.span.end))
            .collect();
        assert_eq!(got, [(0, 1), (3, 4)]);

        let src = "color: 1px red;";
        let got: Vec<_> = tokenize_value(src, sp(7, 14))
            .iter()
            .map(|t| (t.span.start, t.span.end, t.raw))
            .collect();
        assert_eq!(got, [(7, 10, "1px"), (11, 14, "red")]);
        let got: Vec<_> = tokenize_value(src, sp(7, 100))
            .iter()
            .map(|t| t.raw)
            .collect();
        assert_eq!(got, ["1px", "red", ";"]);
        assert!(tokenize_value(src, sp(7, 7)).is_empty());
        assert!(tokenize_value(src, sp(9, 3)).is_empty());
        assert!(tokenize_value(src, sp(50, 60)).is_empty());
        assert_eq!(tokens("12345", sp(0, 2)), [(ValueTokenKind::Number, "12")]);
        assert_eq!(tokens("'abc'", sp(0, 3)), [(ValueTokenKind::String, "'ab")]);
    }

    fn comment(start: u32, end: u32) -> Comment<'static> {
        Comment {
            content: "",
            kind: CommentKind::Block,
            span: sp(start, end),
        }
    }

    #[test]
    fn analyze_preceding_gap_positions() {
        let g = analyze_preceding_gap("a {}", &[], 0);
        assert!(g.is_first_in_file && !g.is_first_nested && !g.after_comment && !g.prev_was_block);
        assert_eq!(
            (
                g.newline_count,
                g.gap_start,
                g.indent_start,
                g.indent_slice.as_str()
            ),
            (0, 0, 0, "")
        );

        let g = analyze_preceding_gap("\n\n  a {}", &[], 4);
        assert!(g.is_first_in_file);
        assert_eq!(
            (
                g.newline_count,
                g.gap_start,
                g.indent_start,
                g.indent_slice.as_str()
            ),
            (2, 0, 2, "  ")
        );

        let g = analyze_preceding_gap("a {}\n\nb {}", &[], 6);
        assert!(!g.is_first_in_file && g.prev_was_block && !g.is_first_nested);
        assert_eq!(
            (
                g.newline_count,
                g.gap_start,
                g.indent_start,
                g.prev_statement_head.as_str()
            ),
            (2, 4, 6, "")
        );

        let g = analyze_preceding_gap("a {}\r\n\r\nb {}", &[], 8);
        assert_eq!((g.newline_count, g.gap_start, g.indent_start), (2, 4, 8));

        let src = "a {\n  color: red;\n  top: 0;\n}";
        let g = analyze_preceding_gap(src, &[], 20);
        assert!(!g.is_first_nested && !g.prev_was_block);
        assert_eq!(
            (
                g.newline_count,
                g.gap_start,
                g.indent_start,
                g.indent_slice.as_str(),
                g.prev_statement_head.as_str()
            ),
            (1, 17, 18, "  ", "color")
        );

        let g = analyze_preceding_gap("a {\n\tcolor: red;}", &[], 5);
        assert!(g.is_first_nested && !g.is_first_in_file);
        assert_eq!(
            (
                g.newline_count,
                g.gap_start,
                g.indent_start,
                g.indent_slice.as_str()
            ),
            (1, 3, 4, "\t")
        );

        let g = analyze_preceding_gap("a{b:c}", &[], 2);
        assert!(g.is_first_nested);
        assert_eq!((g.newline_count, g.gap_start, g.indent_start), (0, 2, 2));

        let g = analyze_preceding_gap("a;", &[], 99);
        assert_eq!(
            (g.gap_start, g.indent_start, g.prev_statement_head.as_str()),
            (2, 2, "a")
        );
    }

    #[test]
    fn analyze_preceding_gap_statement_head() {
        let cases: &[(&str, &str)] = &[
            ("@import 'a';\n@import 'b';", "@import"),
            ("$var: 1;\nb", "$var"),
            ("a { color: red;\nb", "color"),
            ("@include foo(a; b);\nx", "@include"),
            ("a {} @use \"x\" as y;\nb", "@use"),
            ("x { --a:b;\n}", "--a"),
            ("x { ;\ny", ""),
        ];
        for (src, head) in cases {
            let node = src.rfind('\n').unwrap() as u32 + 1;
            let g = analyze_preceding_gap(src, &[], node);
            assert_eq!(g.prev_statement_head, *head, "{src:?}");
        }
    }

    #[test]
    fn analyze_preceding_gap_after_comment() {
        let src = "/* c */\nfoo {}";
        let g = analyze_preceding_gap(src, &[comment(0, 7)], 8);
        assert!(g.after_comment && !g.is_first_in_file);
        assert_eq!(g.gap_start, 7);

        let src = "/*a*/ x; /*b*/\ny";
        assert!(analyze_preceding_gap(src, &[comment(0, 5), comment(9, 14)], 15).after_comment);
        let src = "/*a*/ x;\ny";
        assert!(!analyze_preceding_gap(src, &[comment(0, 5)], 9).after_comment);
        let src = "x; /*a*/y";
        assert!(analyze_preceding_gap(src, &[comment(3, 8)], 8).after_comment);
        assert!(!analyze_preceding_gap("/*a*/", &[comment(0, 5)], 0).after_comment);
    }

    #[test]
    fn analyze_preceding_gap_is_after_comment() {
        let src = "\n      /* comment */\n      foo {}\n    ";
        let c = comment(7, 20);
        assert_eq!(c.span.slice(src), "/* comment */");
        assert!(analyze_preceding_gap(src, &[c], src.find("foo").unwrap() as u32).after_comment);

        let src = "\n      /* comment\n         and more comment */\n      foo {}\n    ";
        let start = src.find("/*").unwrap() as u32;
        let end = src.find("*/").unwrap() as u32 + 2;
        assert!(
            analyze_preceding_gap(src, &[comment(start, end)], src.find("foo").unwrap() as u32)
                .after_comment
        );

        let src = "\n      bar {}\n      foo {}\n    ";
        assert!(!analyze_preceding_gap(src, &[], src.find("foo").unwrap() as u32).after_comment);

        let src = "\n      foo {}\n    ";
        let g = analyze_preceding_gap(src, &[], src.find("foo").unwrap() as u32);
        assert!(!g.after_comment && g.is_first_in_file);
    }

    #[test]
    fn analyze_preceding_gap_ignores_shared_line_block_comment() {
        let src = "\n      bar {} /* comment */\n      foo {}\n    ";
        let start = src.find("/*").unwrap() as u32;
        let end = src.find("*/").unwrap() as u32 + 2;
        assert!(
            !analyze_preceding_gap(src, &[comment(start, end)], src.find("foo").unwrap() as u32)
                .after_comment
        );
    }

    #[test]
    fn analyze_preceding_gap_has_empty_line() {
        let cases: &[(&str, bool)] = &[
            ("\n\n", true),
            ("\r\n\r\n", true),
            ("\n\n\n\n", true),
            ("\r\n\r\n\r\n\r\n", true),
            ("   \n\n", true),
            ("\n\n   \n\n", true),
            ("\n \n", true),
            ("\r\n \r\n", true),
            ("\n \n \n \n", true),
            ("\r \n\n r\n\r\n\r\n", true),
            ("   \n \n", true),
            ("\n \n   \n \n", true),
            ("\n\t\n", true),
            ("\n\t \n", true),
            ("\r\n\t\r\n", true),
            ("\n\t\n\n \t\n", true),
            ("\r\n\t\r\n\t\r\n\r\n", true),
            ("   \n\t\n", true),
            ("\n\t\n  \t  \n\n", true),
            ("", false),
            (" ", false),
            ("\t", false),
            ("\n", false),
            ("\r\n", false),
        ];
        for (gap, expected) in cases {
            let src = format!("a;{gap}b");
            let g = analyze_preceding_gap(&src, &[], src.len() as u32 - 1);
            assert_eq!(g.newline_count >= 2, *expected, "{gap:?}");
        }
    }

    fn decls(src: &str) -> Vec<(&str, bool, &str)> {
        let open = src.find('{').unwrap() as u32;
        scan_direct_declarations(src, sp(open, src.len() as u32))
            .into_iter()
            .map(|d| (d.prop, d.important, d.span.slice(src)))
            .collect()
    }

    #[test]
    fn scan_direct_declarations_cases() {
        let cases: &[DeclCase] = &[
            ("{}", &[]),
            ("{ }", &[]),
            (
                "a { color: red; top: 0 !important }",
                &[
                    ("color", false, "color: red;"),
                    ("top", true, "top: 0 !important"),
                ],
            ),
            (
                "a { color: red; b { top: 0; } left: 1px; }",
                &[
                    ("color", false, "color: red;"),
                    ("left", false, "left: 1px;"),
                ],
            ),
            (
                "a { @include x; color: red }",
                &[("color", false, "color: red")],
            ),
            (
                "a { @media x { top: 0 } color: red }",
                &[("color", false, "color: red")],
            ),
            (
                "a { content: 'a;b}'; top: 0 }",
                &[
                    ("content", false, "content: 'a;b}';"),
                    ("top", false, "top: 0"),
                ],
            ),
            (
                "a { content: \"a\\\";b\"; top: 0 }",
                &[
                    ("content", false, "content: \"a\\\";b\";"),
                    ("top", false, "top: 0"),
                ],
            ),
            (
                "a { background: url(data:a;b{}); top: 0 }",
                &[
                    ("background", false, "background: url(data:a;b{});"),
                    ("top", false, "top: 0"),
                ],
            ),
            (
                "a { color: /* ; */ red; }",
                &[("color", false, "color: /* ; */ red;")],
            ),
            ("a { foo; color: red }", &[("color", false, "color: red")]),
            ("a { b c: d }", &[]),
            ("a { : d }", &[]),
            (
                "a { top: 0 !IMPORTANT; }",
                &[("top", true, "top: 0 !IMPORTANT;")],
            ),
            (
                "a { top: 0!important;; }",
                &[("top", true, "top: 0!important;")],
            ),
            (
                "a { top: 0 ! important }",
                &[("top", false, "top: 0 ! important")],
            ),
            (
                "a {\r\n  color: red;\r\n  top: 0\r\n}",
                &[("color", false, "color: red;"), ("top", false, "top: 0")],
            ),
            ("a { --föo: 'ü'; }", &[("--föo", false, "--föo: 'ü';")]),
            ("a { $var: 1; @var: 2; }", &[("$var", false, "$var: 1;")]),
            (
                "a { #{$p}: 1px; color: red }",
                &[("color", false, "color: red")],
            ),
            ("a { a:hover { x: y } }", &[]),
        ];
        for (src, expected) in cases {
            assert_eq!(&decls(src), expected, "{src:?}");
        }
        let src = "a { color: red; top: 0 !important }";
        let got: Vec<_> = scan_direct_declarations(src, sp(2, 35))
            .iter()
            .map(|d| (d.span.start, d.span.end))
            .collect();
        assert_eq!(got, [(4, 15), (16, 33)]);
        assert!(scan_direct_declarations(src, sp(2, 3)).is_empty());
        assert!(scan_direct_declarations(src, sp(30, 10)).is_empty());
    }

    #[test]
    fn scan_direct_declarations_important() {
        let cases: &[(&str, bool)] = &[
            ("color: pink !important", true),
            ("color: pink !IMPORTANT", true),
            ("color: pink", false),
            ("color: important", false),
            ("color: !importanttt", false),
        ];
        for (decl, expected) in cases {
            let src = format!("{{{decl}}}");
            assert_eq!(decls(&src), [("color", *expected, *decl)], "{decl:?}");
        }
    }

    #[test]
    #[ignore = "diverges from stylelint: whitespace between `!` and `important` is not recognised"]
    fn scan_direct_declarations_important_with_whitespace() {
        for decl in ["color: pink !   important", "color: pink !   IMPORTANT"] {
            let src = format!("{{{decl}}}");
            assert_eq!(decls(&src), [("color", true, decl)], "{decl:?}");
        }
    }

    #[test]
    #[ignore = "diverges from stylelint: a declaration preceded by a comment is dropped"]
    fn scan_direct_declarations_after_comment() {
        assert_eq!(
            decls("a { /* c */ color: red; }"),
            [("color", false, "color: red;")]
        );
    }

    fn items(src: &str) -> Vec<(SelectorItemKind, &str, &str)> {
        scan_selector_items(src, whole(src))
            .into_iter()
            .map(|i| (i.kind, i.name, i.span.slice(src)))
            .collect()
    }

    #[test]
    fn scan_selector_items_kinds() {
        use SelectorItemKind::*;
        let cases: Vec<ItemCase> = vec![
            ("", vec![]),
            ("a", vec![(Type, "a", "a")]),
            (
                "a.b#c",
                vec![(Type, "a", "a"), (Class, "b", ".b"), (Id, "c", "#c")],
            ),
            (
                "a:hover::before",
                vec![
                    (Type, "a", "a"),
                    (
                        PseudoClass {
                            colon_span: sp(1, 2),
                        },
                        "hover",
                        ":hover",
                    ),
                    (
                        PseudoElement {
                            colon_span: sp(7, 9),
                            single_colon: false,
                        },
                        "before",
                        "::before",
                    ),
                ],
            ),
            (
                "a:before :FIRST-LINE",
                vec![
                    (Type, "a", "a"),
                    (
                        PseudoElement {
                            colon_span: sp(1, 2),
                            single_colon: true,
                        },
                        "before",
                        ":before",
                    ),
                    (
                        PseudoElement {
                            colon_span: sp(9, 10),
                            single_colon: true,
                        },
                        "FIRST-LINE",
                        ":FIRST-LINE",
                    ),
                ],
            ),
            (
                "::-webkit-scrollbar",
                vec![(
                    PseudoElement {
                        colon_span: sp(0, 2),
                        single_colon: false,
                    },
                    "-webkit-scrollbar",
                    "::-webkit-scrollbar",
                )],
            ),
            (
                "[data-x='a]b'] > .c",
                vec![
                    (
                        Attribute {
                            inner_span: sp(1, 13),
                        },
                        "data-x='a]b'",
                        "[data-x='a]b']",
                    ),
                    (Class, "c", ".c"),
                ],
            ),
            (
                "[x=\"a\\\"]\"]",
                vec![(
                    Attribute {
                        inner_span: sp(1, 9),
                    },
                    "x=\"a\\\"]\"",
                    "[x=\"a\\\"]\"]",
                )],
            ),
            (
                "[x",
                vec![(
                    Attribute {
                        inner_span: sp(1, 2),
                    },
                    "x",
                    "[x",
                )],
            ),
            (
                "[]",
                vec![(
                    Attribute {
                        inner_span: sp(1, 1),
                    },
                    "",
                    "[]",
                )],
            ),
            (
                "a:not(.b, #c)",
                vec![
                    (Type, "a", "a"),
                    (
                        PseudoClass {
                            colon_span: sp(1, 2),
                        },
                        "not",
                        ":not",
                    ),
                    (Class, "b", ".b"),
                    (Id, "c", "#c"),
                ],
            ),
            (
                "*|a, &.b ~ c + d > e",
                vec![
                    (Type, "a", "a"),
                    (Class, "b", ".b"),
                    (Type, "c", "c"),
                    (Type, "d", "d"),
                    (Type, "e", "e"),
                ],
            ),
            ("/* .x */ .y /* open", vec![(Class, "y", ".y")]),
            ("\"str\" .z", vec![(Class, "z", ".z")]),
            (". # : ::", vec![]),
            (
                ".a-b_cüd -moz-x --y",
                vec![
                    (Class, "a-b_cüd", ".a-b_cüd"),
                    (Type, "-moz-x", "-moz-x"),
                    (Type, "--y", "--y"),
                ],
            ),
            ("a\r\n> b", vec![(Type, "a", "a"), (Type, "b", "b")]),
            (
                "a:nth-child(2n+1)",
                vec![
                    (Type, "a", "a"),
                    (
                        PseudoClass {
                            colon_span: sp(1, 2),
                        },
                        "nth-child",
                        ":nth-child",
                    ),
                    (Type, "n", "n"),
                ],
            ),
            ("#{$sel} .x", vec![(Type, "sel", "sel"), (Class, "x", ".x")]),
        ];
        for (src, expected) in cases {
            assert_eq!(items(src), expected, "{src:?}");
        }
    }

    #[test]
    fn scan_selector_items_spans() {
        let src = "a { } .b, #c { }";
        let got: Vec<_> = scan_selector_items(src, sp(6, 12))
            .iter()
            .map(|i| (i.name, i.span.start, i.span.end))
            .collect();
        assert_eq!(got, [("b", 6, 8), ("c", 10, 12)]);
        assert!(scan_selector_items(src, sp(6, 6)).is_empty());
        assert!(scan_selector_items(src, sp(8, 2)).is_empty());
        let got: Vec<_> = scan_selector_items(".abc", sp(0, 3))
            .iter()
            .map(|i| i.name)
            .collect();
        assert_eq!(got, ["ab"]);
        let got: Vec<_> = scan_selector_items(".abc", sp(0, 99))
            .iter()
            .map(|i| i.name)
            .collect();
        assert_eq!(got, ["abc"]);
    }
}

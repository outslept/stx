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

// ============================================================================
// At-rule prelude scanner (fallback for invalid CSS preludes)
// ============================================================================

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

// ============================================================================
// Value tokenizer
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueTokenKind {
    Ident,
    Number,
    Percentage,
    Dimension { num_len: usize },
    String,
    Function,
    Comma,
    Slash,
    Colon,
    CloseParen,
    Other,
}

#[derive(Debug, Clone)]
pub struct ValueToken<'a> {
    pub kind: ValueTokenKind,
    pub raw: &'a str,
    pub span: Span,
}

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

// ============================================================================
// Preceding gap / empty-line analysis
// ============================================================================

#[derive(Debug, Clone)]
pub struct PreceedingGapInfo {
    pub is_first_in_file: bool,
    pub is_first_nested: bool,
    pub after_comment: bool,
    pub newline_count: usize,
    pub prev_statement_head: String,
    pub prev_was_block: bool,
    pub indent_start: u32,
    pub gap_start: u32,
    pub indent_slice: String,
}

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
    let after_comment = comments
        .get(idx)
        .is_some_and(|c| c.span.end == gap_start_u32);
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

// ============================================================================
// Direct declarations & selector scanners
// ============================================================================

#[derive(Debug, Clone)]
pub struct ScannedDecl<'a> {
    pub prop: &'a str,
    pub important: bool,
    pub span: Span,
}

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
    if brace_depth == 0 && paren_depth == 0 && stmt_start < end {
        if let Some(d) = parse_scanned_decl(source, stmt_start, end) {
            decls.push(d);
        }
    }
    decls
}

fn parse_scanned_decl<'a>(source: &'a str, start: usize, end: usize) -> Option<ScannedDecl<'a>> {
    let (decl_start, decl_end, trimmed) = trim_subslice(source, start, end);
    if trimmed.is_empty() || trimmed.starts_with('@') || trimmed.starts_with("/*") {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorItemKind {
    Type,
    Class,
    Id,
    PseudoClass {
        colon_span: Span,
    },
    PseudoElement {
        colon_span: Span,
        single_colon: bool,
    },
    Attribute {
        inner_span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct ScannedSelectorItem<'a> {
    pub kind: SelectorItemKind,
    pub name: &'a str,
    pub span: Span,
}

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

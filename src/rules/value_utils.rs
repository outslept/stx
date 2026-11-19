use oxc_css_parser::Span;

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

pub fn extract_function_name_and_args<'a>(
    source: &'a str,
    func_span: Span,
) -> Option<(&'a str, Span, Span)> {
    let start = func_span.start as usize;
    let end = (func_span.end as usize).min(source.len());
    if start >= end {
        return None;
    }
    let raw = &source[start..end];
    let paren_offset = raw.find('(')?;
    let fn_name = &raw[..paren_offset];
    if fn_name.is_empty() {
        return None;
    }
    let name_span = Span {
        start: func_span.start,
        end: func_span.start + paren_offset as u32,
    };
    let args_start = name_span.end + 1;
    let args_end = func_span.end.saturating_sub(1);
    if args_start > args_end {
        return None;
    }
    let args_span = Span {
        start: args_start,
        end: args_end,
    };
    Some((fn_name, name_span, args_span))
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
            i += 2;
            while i + 1 < end_idx && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(end_idx);
            continue;
        }

        if b == b'"' || b == b'\'' {
            let quote = b;
            let tok_start = i;
            i += 1;
            while i < end_idx {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(end_idx);
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
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

        if b == b',' {
            tokens.push(ValueToken {
                kind: ValueTokenKind::Comma,
                raw: &source[i..i + 1],
                span: Span {
                    start: i as u32,
                    end: (i + 1) as u32,
                },
            });
            i += 1;
            continue;
        }

        if b == b'/' {
            tokens.push(ValueToken {
                kind: ValueTokenKind::Slash,
                raw: &source[i..i + 1],
                span: Span {
                    start: i as u32,
                    end: (i + 1) as u32,
                },
            });
            i += 1;
            continue;
        }

        if b == b':' {
            tokens.push(ValueToken {
                kind: ValueTokenKind::Colon,
                raw: &source[i..i + 1],
                span: Span {
                    start: i as u32,
                    end: (i + 1) as u32,
                },
            });
            i += 1;
            continue;
        }

        if b == b')' {
            tokens.push(ValueToken {
                kind: ValueTokenKind::CloseParen,
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

            if i < end_idx && bytes[i] == b'%' {
                i += 1;
                tokens.push(ValueToken {
                    kind: ValueTokenKind::Percentage,
                    raw: &source[tok_start..i],
                    span: Span {
                        start: tok_start as u32,
                        end: i as u32,
                    },
                });
            } else if i < end_idx && is_ident_start_byte(bytes[i]) {
                while i < end_idx && is_ident_char_byte(bytes[i]) {
                    i += 1;
                }
                tokens.push(ValueToken {
                    kind: ValueTokenKind::Dimension { num_len },
                    raw: &source[tok_start..i],
                    span: Span {
                        start: tok_start as u32,
                        end: i as u32,
                    },
                });
            } else {
                tokens.push(ValueToken {
                    kind: ValueTokenKind::Number,
                    raw: &source[tok_start..i],
                    span: Span {
                        start: tok_start as u32,
                        end: i as u32,
                    },
                });
            }
            continue;
        }

        if is_ident_start_byte(b) || b == b'-' || b == b'$' || b == b'@' {
            let tok_start = i;
            i += 1;
            while i < end_idx && is_ident_char_byte(bytes[i]) {
                i += 1;
            }
            if i < end_idx && bytes[i] == b'(' {
                i += 1;
                tokens.push(ValueToken {
                    kind: ValueTokenKind::Function,
                    raw: &source[tok_start..i],
                    span: Span {
                        start: tok_start as u32,
                        end: i as u32,
                    },
                });
            } else {
                tokens.push(ValueToken {
                    kind: ValueTokenKind::Ident,
                    raw: &source[tok_start..i],
                    span: Span {
                        start: tok_start as u32,
                        end: i as u32,
                    },
                });
            }
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

pub fn declaration_value_span(
    source: &str,
    name_end: u32,
    decl_end: u32,
    important_start: Option<u32>,
) -> Option<Span> {
    let end = important_start.unwrap_or(decl_end) as usize;
    let start = name_end as usize;
    if start >= end || end > source.len() {
        return None;
    }
    let slice = &source[start..end];
    let colon_offset = slice.find(':')?;
    let val_start = start + colon_offset + 1;
    let mut val_end = end;
    while val_end > val_start {
        let b = source.as_bytes()[val_end - 1];
        if b == b';' || b.is_ascii_whitespace() {
            val_end -= 1;
        } else {
            break;
        }
    }
    if val_start <= val_end {
        Some(Span {
            start: val_start as u32,
            end: val_end as u32,
        })
    } else {
        None
    }
}

pub fn format_clean_float(val: f64) -> String {
    let rounded = (val * 100000.0).round() / 100000.0;
    if rounded == 0.0 {
        return "0".to_string();
    }
    let s = format!("{:.5}", rounded);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
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

use oxc_css_parser::token::Comment;

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

    let after_comment = comments.iter().any(|c| c.span.end as usize == gap_start);

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
        gap_start: gap_start as u32,
        indent_slice,
    }
}

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
            i += 2;
            while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(end);
            continue;
        }

        if b == b'"' || b == b'\'' {
            let q = b;
            i += 1;
            while i < end {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(end);
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
    let raw = &source[start..end];
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('@') || trimmed.starts_with("/*") {
        return None;
    }
    let offset = raw.find(trimmed)?;
    let decl_start = start + offset;
    let decl_end = decl_start + trimmed.len();
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
    let important = val_part.to_ascii_lowercase().ends_with("!important");
    Some(ScannedDecl {
        prop,
        important,
        span: Span {
            start: decl_start as u32,
            end: decl_end as u32,
        },
    })
}

pub fn at_rule_prelude_span(
    source: &str,
    at_rule: &oxc_css_parser::ast::AtRule<'_>,
) -> Option<Span> {
    let start = at_rule.name.span.end as usize;
    let raw_end = match &at_rule.block {
        Some(b) => b.span.start as usize,
        None => at_rule.span.end as usize,
    }
    .min(source.len());
    if start >= raw_end {
        return None;
    }
    let slice = &source[start..raw_end];
    let trimmed = slice
        .trim_end_matches(|c: char| c.is_ascii_whitespace() || c == ';')
        .trim();
    if trimmed.is_empty() {
        return None;
    }
    let offset = slice.find(trimmed)?;
    let p_start = start + offset;
    let p_end = p_start + trimmed.len();
    Some(Span {
        start: p_start as u32,
        end: p_end as u32,
    })
}

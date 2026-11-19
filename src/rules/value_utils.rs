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

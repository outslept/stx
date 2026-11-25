use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rules::Rule;
use crate::utils::ast::decl_value_raw;
use oxc_css_parser::Span;
use oxc_css_parser::ast::Declaration;
use regex::Regex;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Never,
    AlwaysWherePossible,
}

#[derive(Debug, Clone)]
enum StringOrRegex {
    Exact(String),
    Regex(Regex),
}

impl StringOrRegex {
    fn parse(s: &str) -> Option<Self> {
        if s.starts_with('/')
            && s.len() >= 2
            && let Some(last_slash) = s[1..].rfind('/')
        {
            let pattern = &s[1..=last_slash];
            let flags = &s[last_slash + 2..];
            let full_pattern = if flags.contains('i') {
                format!("(?i){pattern}")
            } else {
                pattern.to_string()
            };
            if let Ok(re) = Regex::new(&full_pattern) {
                return Some(Self::Regex(re));
            }
        }
        Some(Self::Exact(s.to_string()))
    }

    fn is_match(&self, candidate: &str) -> bool {
        match self {
            Self::Exact(s) => s == candidate,
            Self::Regex(re) => re.is_match(candidate),
        }
    }
}

crate::define_stx_lint! {
    /// Forbid named colors or require them where an equivalent name is known.
    ///
    /// Use `"never"` to disallow color names or `"always-where-possible"` to
    /// prefer names over equivalent hex colors and supported color functions.
    /// The rule skips properties such as `animation-name` where a color word
    /// may instead be a custom identifier.
    ///
    /// `ignoreProperties` and `ignoreFunctions` take names or slash-delimited
    /// regexes. `ignore: ["inside-function"]` skips whole function calls,
    /// including the call itself as a candidate for replacement.
    ///
    /// With `"always-where-possible"`:
    /// ```css
    /// a { color: #f00; } /* reported */
    /// a { color: red; }  /* accepted */
    /// ```
    "color-named", style;
    pub struct ColorNamed {
        pub mode: Mode,
        pub ignore_inside_function: bool,
        ignore_properties: Vec<StringOrRegex>,
        ignore_functions: Vec<StringOrRegex>,
        pub severity: Severity,
    }
}

impl ColorNamed {
    pub fn from_config(
        primary: &Value,
        secondary: Option<&Map<String, Value>>,
        severity: Severity,
    ) -> Option<Self> {
        let mode = match primary.as_str()? {
            "never" => Mode::Never,
            "always-where-possible" => Mode::AlwaysWherePossible,
            _ => return None,
        };

        let mut ignore_inside_function = false;
        let mut ignore_properties = Vec::new();
        let mut ignore_functions = Vec::new();

        if let Some(sec) = secondary {
            if let Some(arr) = sec.get("ignore").and_then(|v| v.as_array()) {
                for item in arr {
                    if item.as_str() == Some("inside-function") {
                        ignore_inside_function = true;
                    }
                }
            }
            if let Some(arr) = sec.get("ignoreProperties").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str()
                        && let Some(matcher) = StringOrRegex::parse(s)
                    {
                        ignore_properties.push(matcher);
                    }
                }
            }
            if let Some(arr) = sec.get("ignoreFunctions").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str()
                        && let Some(matcher) = StringOrRegex::parse(s)
                    {
                        ignore_functions.push(matcher);
                    }
                }
            }
        }

        Some(Self {
            mode,
            ignore_inside_function,
            ignore_properties,
            ignore_functions,
            severity,
        })
    }

    fn is_ignored_property(&self, prop: &str) -> bool {
        self.ignore_properties.iter().any(|m| m.is_match(prop))
    }

    fn is_ignored_function(&self, func_name: &str) -> bool {
        self.ignore_functions.iter().any(|m| m.is_match(func_name))
    }
}

pub fn expected_msg(actual: &str, expected: &str) -> String {
    format!("Expected \"{actual}\" to be \"{expected}\"")
}

pub fn rejected_msg(keyword: &str) -> String {
    format!("Disallowed named color \"{keyword}\"")
}

impl Rule for ColorNamed {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let prop = ctx
            .slice_range(decl.span.start, decl.colon_span.start)
            .trim();
        if accepts_custom_idents(prop) {
            return;
        }
        if self.is_ignored_property(prop) {
            return;
        }

        let Some((val_raw, val_span)) = decl_value_raw(ctx.source, decl) else {
            return;
        };

        let nodes = parse_value_nodes(val_raw);
        self.walk_nodes(&nodes, val_raw, val_span.start, ctx);
    }
}

impl ColorNamed {
    fn walk_nodes<'a>(
        &self,
        nodes: &[ValueNode],
        raw_source: &str,
        base_offset: u32,
        ctx: &mut LintContext<'a>,
    ) {
        for node in nodes {
            match node {
                ValueNode::Word { start, end } => {
                    let word = &raw_source[*start..*end];
                    if !is_standard_syntax_value(word) {
                        continue;
                    }

                    match self.mode {
                        Mode::Never => {
                            let lower = word.to_ascii_lowercase();
                            if is_named_color(&lower) {
                                let span = Span {
                                    start: base_offset + *start as u32,
                                    end: base_offset + *end as u32,
                                };
                                ctx.report(self.name(), span, rejected_msg(word));
                            }
                        }
                        Mode::AlwaysWherePossible => {
                            if word.starts_with('#')
                                && let Some(rgba) = parse_color_string(word)
                                && let Some(named) = rgba_to_name(rgba)
                            {
                                let span = Span {
                                    start: base_offset + *start as u32,
                                    end: base_offset + *end as u32,
                                };
                                ctx.report(self.name(), span, expected_msg(word, named));
                            }
                        }
                    }
                }
                ValueNode::Function {
                    name_start,
                    name_end,
                    full_start,
                    full_end,
                    children,
                } => {
                    let func_name = &raw_source[*name_start..*name_end];
                    if self.ignore_inside_function {
                        continue;
                    }
                    if self.is_ignored_function(func_name) {
                        continue;
                    }
                    if !is_standard_syntax_function(func_name) {
                        continue;
                    }
                    if !is_standard_syntax_value(func_name) {
                        continue;
                    }

                    if self.mode == Mode::AlwaysWherePossible {
                        let raw_color_string = &raw_source[*full_start..*full_end];
                        let color_string = normalize_color_function_string(raw_color_string);
                        if let Some(rgba) = parse_color_string(&color_string)
                            && let Some(named) = rgba_to_name(rgba)
                        {
                            let span = Span {
                                start: base_offset + *full_start as u32,
                                end: base_offset + *full_end as u32,
                            };
                            ctx.report(self.name(), span, expected_msg(&color_string, named));
                        }
                    }

                    self.walk_nodes(children, raw_source, base_offset, ctx);
                }
            }
        }
    }
}

fn accepts_custom_idents(prop: &str) -> bool {
    let lower = prop.to_ascii_lowercase();
    let unprefixed = strip_vendor_prefix(&lower);
    matches!(
        unprefixed,
        "animation"
            | "animation-name"
            | "counter-increment"
            | "counter-reset"
            | "counter-set"
            | "font"
            | "font-family"
            | "grid-area"
            | "grid-column"
            | "grid-column-end"
            | "grid-column-start"
            | "grid-row"
            | "grid-row-end"
            | "grid-row-start"
            | "list-style"
            | "list-style-type"
            | "transition"
            | "transition-property"
            | "view-transition-name"
            | "will-change"
    )
}

fn strip_vendor_prefix(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix('-')
        && !rest.starts_with('-')
        && let Some(idx) = rest.find('-')
    {
        return &rest[idx + 1..];
    }
    s
}

fn is_standard_syntax_function(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.starts_with("#{") || name.starts_with("${") || name.starts_with('`') {
        return false;
    }
    true
}

fn is_standard_syntax_value(value: &str) -> bool {
    if value.starts_with('$') || value.starts_with('@') {
        return false;
    }
    if value.contains("#{") || value.contains("${") {
        return false;
    }
    true
}

#[derive(Debug)]
enum ValueNode {
    Word {
        start: usize,
        end: usize,
    },
    Function {
        name_start: usize,
        name_end: usize,
        full_start: usize,
        full_end: usize,
        children: Vec<ValueNode>,
    },
}

fn parse_value_nodes(s: &str) -> Vec<ValueNode> {
    let bytes = s.as_bytes();
    let (nodes, _) = parse_value_slice(s, bytes, 0, bytes.len(), false);
    nodes
}

fn parse_value_slice(
    s: &str,
    bytes: &[u8],
    mut i: usize,
    limit: usize,
    stop_at_close_paren: bool,
) -> (Vec<ValueNode>, usize) {
    let mut nodes = Vec::new();
    while i < limit {
        let b = bytes[i];

        if stop_at_close_paren && b == b')' {
            return (nodes, i + 1);
        }

        // Whitespace or dividers
        if b.is_ascii_whitespace() || matches!(b, b',' | b'/' | b':') {
            if b == b'/' && i + 1 < limit && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < limit && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(limit);
                continue;
            }
            if b == b'/' && i + 1 < limit && bytes[i + 1] == b'/' {
                i += 2;
                while i < limit && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            i += 1;
            continue;
        }

        // Strings
        if b == b'\'' || b == b'"' {
            let quote = b;
            i += 1;
            while i < limit {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(limit);
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

        // Parenthesized block without function name
        if b == b'(' {
            let full_start = i;
            let (children, next_i) = parse_value_slice(s, bytes, i + 1, limit, true);
            nodes.push(ValueNode::Function {
                name_start: full_start,
                name_end: full_start,
                full_start,
                full_end: next_i,
                children,
            });
            i = next_i;
            continue;
        }

        // Word or function
        let word_start = i;
        while i < limit {
            let c = bytes[i];
            if c == b'\\' {
                i = (i + 2).min(limit);
                continue;
            }
            if c.is_ascii_whitespace()
                || matches!(c, b',' | b'/' | b':' | b'(' | b')' | b'\'' | b'"')
            {
                break;
            }
            i += 1;
        }
        let word_end = i;

        if i < limit && bytes[i] == b'(' {
            let func_name = &s[word_start..word_end];
            if func_name.eq_ignore_ascii_case("url") {
                let (children, next_i) = parse_url_args(bytes, i + 1, limit);
                nodes.push(ValueNode::Function {
                    name_start: word_start,
                    name_end: word_end,
                    full_start: word_start,
                    full_end: next_i,
                    children,
                });
                i = next_i;
            } else {
                let (children, next_i) = parse_value_slice(s, bytes, i + 1, limit, true);
                nodes.push(ValueNode::Function {
                    name_start: word_start,
                    name_end: word_end,
                    full_start: word_start,
                    full_end: next_i,
                    children,
                });
                i = next_i;
            }
        } else if word_end > word_start {
            nodes.push(ValueNode::Word {
                start: word_start,
                end: word_end,
            });
        } else {
            i += 1;
        }
    }
    (nodes, i)
}

fn parse_url_args(bytes: &[u8], mut i: usize, limit: usize) -> (Vec<ValueNode>, usize) {
    while i < limit && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < limit && (bytes[i] == b'\'' || bytes[i] == b'"') {
        let quote = bytes[i];
        i += 1;
        while i < limit {
            if bytes[i] == b'\\' {
                i = (i + 2).min(limit);
                continue;
            }
            if bytes[i] == quote {
                i += 1;
                break;
            }
            i += 1;
        }
        while i < limit && bytes[i] != b')' {
            i += 1;
        }
        let end = if i < limit && bytes[i] == b')' {
            i + 1
        } else {
            i
        };
        return (Vec::new(), end);
    }

    let arg_start = i;
    while i < limit && bytes[i] != b')' {
        if bytes[i] == b'\\' {
            i = (i + 2).min(limit);
            continue;
        }
        i += 1;
    }
    let mut arg_end = i;
    while arg_end > arg_start && bytes[arg_end - 1].is_ascii_whitespace() {
        arg_end -= 1;
    }
    let next_i = if i < limit && bytes[i] == b')' {
        i + 1
    } else {
        i
    };
    let mut children = Vec::new();
    if arg_end > arg_start {
        children.push(ValueNode::Word {
            start: arg_start,
            end: arg_end,
        });
    }
    (children, next_i)
}

fn normalize_color_function_string(raw: &str) -> String {
    // Equivalent to: raw.replace(/\s*([,/()])\s*/g, '$1').replace(/\s{2,}/g, ' ')
    let mut step1 = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && matches!(chars[j], ',' | '/' | '(' | ')') {
                step1.push(chars[j]);
                i = j + 1;
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                continue;
            } else {
                step1.push(' ');
                i = j;
                continue;
            }
        }
        if matches!(chars[i], ',' | '/' | '(' | ')') {
            step1.push(chars[i]);
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            continue;
        }
        step1.push(chars[i]);
        i += 1;
    }
    step1
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: f64,
}

fn parse_color_string(input: &str) -> Option<Rgba> {
    let s = input.trim();
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    let open_paren = s.find('(')?;
    if !s.ends_with(')') {
        return None;
    }
    let func_name = s[..open_paren].to_ascii_lowercase();
    let args = s[open_paren + 1..s.len() - 1].trim();

    match func_name.as_str() {
        "rgb" | "rgba" => parse_rgb_function(args),
        "hsl" | "hsla" => parse_hsl_function(args),
        "hwb" => parse_hwb_function(args),
        "gray" => parse_gray_function(args),
        _ => None,
    }
}

fn parse_hex_color(hex: &str) -> Option<Rgba> {
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Rgba { r, g, b, a: 1.0 })
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            let a = u8::from_str_radix(&hex[3..4], 16).ok()? * 17;
            Some(Rgba {
                r,
                g,
                b,
                a: (a as f64) / 255.0,
            })
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Rgba { r, g, b, a: 1.0 })
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Rgba {
                r,
                g,
                b,
                a: (a as f64) / 255.0,
            })
        }
        _ => None,
    }
}

fn split_func_args(args: &str) -> Option<(Vec<&str>, Option<&str>, bool)> {
    if args.contains(',') {
        if args.contains('/') {
            return None;
        }
        let parts: Vec<&str> = args.split(',').map(str::trim).collect();
        if parts.iter().any(|p| p.is_empty()) {
            return None;
        }
        match parts.len() {
            3 => Some((vec![parts[0], parts[1], parts[2]], None, true)),
            4 => Some((vec![parts[0], parts[1], parts[2]], Some(parts[3]), true)),
            _ => None,
        }
    } else if let Some((left, right)) = args.split_once('/') {
        if right.contains('/') {
            return None;
        }
        let channels: Vec<&str> = left.split_whitespace().collect();
        let alpha = right.trim();
        if channels.len() != 3 || alpha.is_empty() {
            return None;
        }
        Some((channels, Some(alpha), false))
    } else {
        let channels: Vec<&str> = args.split_whitespace().collect();
        if channels.len() != 3 {
            return None;
        }
        Some((channels, None, false))
    }
}

fn parse_number_or_percent(s: &str) -> Option<(f64, bool)> {
    let s = s.trim();
    if let Some(num_str) = s.strip_suffix('%') {
        let val = num_str.parse::<f64>().ok()?;
        Some((val, true))
    } else {
        let val = s.parse::<f64>().ok()?;
        Some((val, false))
    }
}

fn parse_alpha_value(alpha_opt: Option<&str>) -> Option<f64> {
    let Some(a_str) = alpha_opt else {
        return Some(1.0);
    };
    let (val, is_pct) = parse_number_or_percent(a_str)?;
    let a = if is_pct { val / 100.0 } else { val };
    Some(a.clamp(0.0, 1.0))
}

fn parse_angle_degrees(s: &str) -> Option<f64> {
    let s = s.trim().to_ascii_lowercase();
    let deg = if let Some(v) = s.strip_suffix("deg") {
        v.parse::<f64>().ok()?
    } else if let Some(v) = s.strip_suffix("grad") {
        v.parse::<f64>().ok()? * 0.9
    } else if let Some(v) = s.strip_suffix("rad") {
        v.parse::<f64>().ok()?.to_degrees()
    } else if let Some(v) = s.strip_suffix("turn") {
        v.parse::<f64>().ok()? * 360.0
    } else {
        s.parse::<f64>().ok()?
    };
    Some(deg.rem_euclid(360.0))
}

fn parse_rgb_function(args: &str) -> Option<Rgba> {
    let (channels, alpha_opt, _) = split_func_args(args)?;
    let (r_val, r_pct) = parse_number_or_percent(channels[0])?;
    let (g_val, g_pct) = parse_number_or_percent(channels[1])?;
    let (b_val, b_pct) = parse_number_or_percent(channels[2])?;
    if r_pct != g_pct || g_pct != b_pct {
        return None;
    }
    let to_u8 = |v: f64| -> u8 {
        let scaled = if r_pct { v * 2.55 } else { v };
        scaled.round().clamp(0.0, 255.0) as u8
    };
    let a = parse_alpha_value(alpha_opt)?;
    Some(Rgba {
        r: to_u8(r_val),
        g: to_u8(g_val),
        b: to_u8(b_val),
        a,
    })
}

fn parse_hsl_function(args: &str) -> Option<Rgba> {
    let (channels, alpha_opt, _) = split_func_args(args)?;
    let h = parse_angle_degrees(channels[0])?;
    let (s_val, s_pct) = parse_number_or_percent(channels[1])?;
    let (l_val, l_pct) = parse_number_or_percent(channels[2])?;
    if !s_pct || !l_pct {
        return None;
    }
    let a = parse_alpha_value(alpha_opt)?;
    let (r, g, b) = hsl_to_rgb(
        h,
        s_val.clamp(0.0, 100.0) / 100.0,
        l_val.clamp(0.0, 100.0) / 100.0,
    );
    Some(Rgba { r, g, b, a })
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn parse_hwb_function(args: &str) -> Option<Rgba> {
    let (channels, alpha_opt, _) = split_func_args(args)?;
    let h = parse_angle_degrees(channels[0])?;
    let (w_val, w_pct) = parse_number_or_percent(channels[1])?;
    let (b_val, b_pct) = parse_number_or_percent(channels[2])?;
    if !w_pct || !b_pct {
        return None;
    }
    let a = parse_alpha_value(alpha_opt)?;
    let mut w = w_val.clamp(0.0, 100.0) / 100.0;
    let mut bl = b_val.clamp(0.0, 100.0) / 100.0;
    if w + bl > 1.0 {
        let sum = w + bl;
        w /= sum;
        bl /= sum;
    }
    let (r0, g0, b0) = hsl_to_rgb(h, 1.0, 0.5);
    let conv = |ch: u8| -> u8 {
        let v = (ch as f64 / 255.0) * (1.0 - w - bl) + w;
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    };
    Some(Rgba {
        r: conv(r0),
        g: conv(g0),
        b: conv(b0),
        a,
    })
}

fn parse_gray_function(args: &str) -> Option<Rgba> {
    if args.contains('/') || args.is_empty() {
        return None;
    }
    let parts: Vec<&str> = args.split(',').collect();
    if parts.is_empty() || parts.len() > 2 {
        return None;
    }
    let l_str = parts[0].trim();
    if l_str.is_empty() {
        return None;
    }
    let (l, _) = parse_number_or_percent(l_str)?;
    let a = if parts.len() == 2 {
        let a_str = parts[1].trim();
        if a_str.is_empty() {
            return None;
        }
        parse_alpha_value(Some(a_str))?
    } else {
        1.0
    };

    // CIE Lab (l, 0, 0) -> sRGB
    let y = (l + 16.0) / 116.0;
    let y3 = y * y * y;
    let xyz_y = if y3 > 0.008856 {
        y3
    } else {
        (116.0 * y - 16.0) / 903.3
    };
    let srgb = if xyz_y > 0.0031308 {
        1.055 * xyz_y.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * xyz_y
    };
    let v = (srgb * 255.0).round().clamp(0.0, 255.0) as u8;
    Some(Rgba {
        r: v,
        g: v,
        b: v,
        a,
    })
}

fn rgba_to_name(c: Rgba) -> Option<&'static str> {
    if (c.a - 1.0).abs() > f64::EPSILON {
        return None;
    }
    match (c.r, c.g, c.b) {
        (255, 255, 255) => Some("white"),
        (0, 0, 0) => Some("black"),
        (255, 0, 0) => Some("red"),
        (0, 255, 0) => Some("lime"),
        (0, 0, 255) => Some("blue"),
        (255, 255, 0) => Some("yellow"),
        (0, 255, 255) => Some("cyan"),
        (255, 0, 255) => Some("magenta"),
        (192, 192, 192) => Some("silver"),
        (128, 128, 128) => Some("grey"),
        (128, 0, 0) => Some("maroon"),
        (128, 128, 0) => Some("olive"),
        (0, 128, 0) => Some("green"),
        (128, 0, 128) => Some("purple"),
        (0, 128, 128) => Some("teal"),
        (0, 0, 128) => Some("navy"),
        (240, 248, 255) => Some("aliceblue"),
        (250, 235, 215) => Some("antiquewhite"),
        (127, 255, 212) => Some("aquamarine"),
        (240, 255, 255) => Some("azure"),
        (245, 245, 220) => Some("beige"),
        (255, 228, 196) => Some("bisque"),
        (255, 235, 205) => Some("blanchedalmond"),
        (138, 43, 226) => Some("blueviolet"),
        (165, 42, 42) => Some("brown"),
        (222, 184, 135) => Some("burlywood"),
        (95, 158, 160) => Some("cadetblue"),
        (127, 255, 0) => Some("chartreuse"),
        (210, 105, 30) => Some("chocolate"),
        (255, 127, 80) => Some("coral"),
        (100, 149, 237) => Some("cornflowerblue"),
        (255, 248, 220) => Some("cornsilk"),
        (220, 20, 60) => Some("crimson"),
        (0, 0, 139) => Some("darkblue"),
        (0, 139, 139) => Some("darkcyan"),
        (184, 134, 11) => Some("darkgoldenrod"),
        (169, 169, 169) => Some("darkgrey"),
        (0, 100, 0) => Some("darkgreen"),
        (189, 183, 107) => Some("darkkhaki"),
        (139, 0, 139) => Some("darkmagenta"),
        (85, 107, 47) => Some("darkolivegreen"),
        (255, 140, 0) => Some("darkorange"),
        (153, 50, 204) => Some("darkorchid"),
        (139, 0, 0) => Some("darkred"),
        (233, 150, 122) => Some("darksalmon"),
        (143, 188, 143) => Some("darkseagreen"),
        (72, 61, 139) => Some("darkslateblue"),
        (47, 79, 79) => Some("darkslategrey"),
        (0, 206, 209) => Some("darkturquoise"),
        (148, 0, 211) => Some("darkviolet"),
        (255, 20, 147) => Some("deeppink"),
        (0, 191, 255) => Some("deepskyblue"),
        (105, 105, 105) => Some("dimgrey"),
        (30, 144, 255) => Some("dodgerblue"),
        (178, 34, 34) => Some("firebrick"),
        (255, 250, 240) => Some("floralwhite"),
        (34, 139, 34) => Some("forestgreen"),
        (220, 220, 220) => Some("gainsboro"),
        (248, 248, 255) => Some("ghostwhite"),
        (255, 215, 0) => Some("gold"),
        (218, 165, 32) => Some("goldenrod"),
        (173, 255, 47) => Some("greenyellow"),
        (240, 255, 240) => Some("honeydew"),
        (255, 105, 180) => Some("hotpink"),
        (205, 92, 92) => Some("indianred"),
        (75, 0, 130) => Some("indigo"),
        (255, 255, 240) => Some("ivory"),
        (240, 230, 140) => Some("khaki"),
        (230, 230, 250) => Some("lavender"),
        (255, 240, 245) => Some("lavenderblush"),
        (124, 252, 0) => Some("lawngreen"),
        (255, 250, 205) => Some("lemonchiffon"),
        (173, 216, 230) => Some("lightblue"),
        (240, 128, 128) => Some("lightcoral"),
        (224, 255, 255) => Some("lightcyan"),
        (250, 250, 210) => Some("lightgoldenrodyellow"),
        (211, 211, 211) => Some("lightgrey"),
        (144, 238, 144) => Some("lightgreen"),
        (255, 182, 193) => Some("lightpink"),
        (255, 160, 122) => Some("lightsalmon"),
        (32, 178, 170) => Some("lightseagreen"),
        (135, 206, 250) => Some("lightskyblue"),
        (119, 136, 153) => Some("lightslategrey"),
        (176, 196, 222) => Some("lightsteelblue"),
        (255, 255, 224) => Some("lightyellow"),
        (50, 205, 50) => Some("limegreen"),
        (250, 240, 230) => Some("linen"),
        (102, 205, 170) => Some("mediumaquamarine"),
        (0, 0, 205) => Some("mediumblue"),
        (186, 85, 211) => Some("mediumorchid"),
        (147, 112, 219) => Some("mediumpurple"),
        (60, 179, 113) => Some("mediumseagreen"),
        (123, 104, 238) => Some("mediumslateblue"),
        (0, 250, 154) => Some("mediumspringgreen"),
        (72, 209, 204) => Some("mediumturquoise"),
        (199, 21, 133) => Some("mediumvioletred"),
        (25, 25, 112) => Some("midnightblue"),
        (245, 255, 250) => Some("mintcream"),
        (255, 228, 225) => Some("mistyrose"),
        (255, 228, 181) => Some("moccasin"),
        (255, 222, 173) => Some("navajowhite"),
        (253, 245, 230) => Some("oldlace"),
        (107, 142, 35) => Some("olivedrab"),
        (255, 165, 0) => Some("orange"),
        (255, 69, 0) => Some("orangered"),
        (218, 112, 214) => Some("orchid"),
        (238, 232, 170) => Some("palegoldenrod"),
        (152, 251, 152) => Some("palegreen"),
        (175, 238, 238) => Some("paleturquoise"),
        (219, 112, 147) => Some("palevioletred"),
        (255, 239, 213) => Some("papayawhip"),
        (255, 218, 185) => Some("peachpuff"),
        (205, 133, 63) => Some("peru"),
        (255, 192, 203) => Some("pink"),
        (221, 160, 221) => Some("plum"),
        (176, 224, 230) => Some("powderblue"),
        (102, 51, 153) => Some("rebeccapurple"),
        (188, 143, 143) => Some("rosybrown"),
        (65, 105, 225) => Some("royalblue"),
        (139, 69, 19) => Some("saddlebrown"),
        (250, 128, 114) => Some("salmon"),
        (244, 164, 96) => Some("sandybrown"),
        (46, 139, 87) => Some("seagreen"),
        (255, 245, 238) => Some("seashell"),
        (160, 82, 45) => Some("sienna"),
        (135, 206, 235) => Some("skyblue"),
        (106, 90, 205) => Some("slateblue"),
        (112, 128, 144) => Some("slategrey"),
        (255, 250, 250) => Some("snow"),
        (0, 255, 127) => Some("springgreen"),
        (70, 130, 180) => Some("steelblue"),
        (210, 180, 140) => Some("tan"),
        (216, 191, 216) => Some("thistle"),
        (255, 99, 71) => Some("tomato"),
        (64, 224, 208) => Some("turquoise"),
        (238, 130, 238) => Some("violet"),
        (245, 222, 179) => Some("wheat"),
        (245, 245, 245) => Some("whitesmoke"),
        (154, 205, 50) => Some("yellowgreen"),
        _ => None,
    }
}

fn is_named_color(lower: &str) -> bool {
    matches!(
        lower,
        "aliceblue"
            | "antiquewhite"
            | "aqua"
            | "aquamarine"
            | "azure"
            | "beige"
            | "bisque"
            | "black"
            | "blanchedalmond"
            | "blue"
            | "blueviolet"
            | "brown"
            | "burlywood"
            | "cadetblue"
            | "chartreuse"
            | "chocolate"
            | "coral"
            | "cornflowerblue"
            | "cornsilk"
            | "crimson"
            | "cyan"
            | "darkblue"
            | "darkcyan"
            | "darkgoldenrod"
            | "darkgray"
            | "darkgreen"
            | "darkgrey"
            | "darkkhaki"
            | "darkmagenta"
            | "darkolivegreen"
            | "darkorange"
            | "darkorchid"
            | "darkred"
            | "darksalmon"
            | "darkseagreen"
            | "darkslateblue"
            | "darkslategray"
            | "darkslategrey"
            | "darkturquoise"
            | "darkviolet"
            | "deeppink"
            | "deepskyblue"
            | "dimgray"
            | "dimgrey"
            | "dodgerblue"
            | "firebrick"
            | "floralwhite"
            | "forestgreen"
            | "fuchsia"
            | "gainsboro"
            | "ghostwhite"
            | "gold"
            | "goldenrod"
            | "gray"
            | "green"
            | "greenyellow"
            | "grey"
            | "honeydew"
            | "hotpink"
            | "indianred"
            | "indigo"
            | "ivory"
            | "khaki"
            | "lavender"
            | "lavenderblush"
            | "lawngreen"
            | "lemonchiffon"
            | "lightblue"
            | "lightcoral"
            | "lightcyan"
            | "lightgoldenrodyellow"
            | "lightgray"
            | "lightgreen"
            | "lightgrey"
            | "lightpink"
            | "lightsalmon"
            | "lightseagreen"
            | "lightskyblue"
            | "lightslategray"
            | "lightslategrey"
            | "lightsteelblue"
            | "lightyellow"
            | "lime"
            | "limegreen"
            | "linen"
            | "magenta"
            | "maroon"
            | "mediumaquamarine"
            | "mediumblue"
            | "mediumorchid"
            | "mediumpurple"
            | "mediumseagreen"
            | "mediumslateblue"
            | "mediumspringgreen"
            | "mediumturquoise"
            | "mediumvioletred"
            | "midnightblue"
            | "mintcream"
            | "mistyrose"
            | "moccasin"
            | "navajowhite"
            | "navy"
            | "oldlace"
            | "olive"
            | "olivedrab"
            | "orange"
            | "orangered"
            | "orchid"
            | "palegoldenrod"
            | "palegreen"
            | "paleturquoise"
            | "palevioletred"
            | "papayawhip"
            | "peachpuff"
            | "peru"
            | "pink"
            | "plum"
            | "powderblue"
            | "purple"
            | "rebeccapurple"
            | "red"
            | "rosybrown"
            | "royalblue"
            | "saddlebrown"
            | "salmon"
            | "sandybrown"
            | "seagreen"
            | "seashell"
            | "sienna"
            | "silver"
            | "skyblue"
            | "slateblue"
            | "slategray"
            | "slategrey"
            | "snow"
            | "springgreen"
            | "steelblue"
            | "tan"
            | "teal"
            | "thistle"
            | "tomato"
            | "turquoise"
            | "violet"
            | "wheat"
            | "white"
            | "whitesmoke"
            | "yellow"
            | "yellowgreen"
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject};

    const NAME: &str = "color-named";

    fn expected(actual: &str, expected: &str) -> String {
        format!("Expected \"{actual}\" to be \"{expected}\"")
    }

    fn rejected(keyword: &str) -> String {
        format!("Disallowed named color \"{keyword}\"")
    }

    #[test]
    fn never() {
        let pass = &[
            "a { color: #000; }",
            "a { color: #ababab; }",
            "a { color: #AbAbAb; }",
            "a { color: #ABABAB; }",
            "a { color: rgba(0, 0, 0, 0); }",
            "a { background: #000, #fff, #333; }",
            "/** color: black; */",
            "a::before { content: \"orange\" }",
            "a::before { content: 'orange' }",
            "a { background-image: url(./black.png); }",
            "a { padding: 000; }",
            "a::before { content: \"#ababa\"; }",
            "a { color: $black; }",
            "a { color: var(--some-color-blue); }",
            "a { animation: spin-blue 2s linear; }",
            "a { animation: blue 2s linear; }",
            "a { font-family: blue; }",
            "a { font: 10px/14px Brown, sans-serif; }",
            "$colors: (white: #fff, blue: #00f);",
            "a { color: transparent; }",
        ];

        let fail = &[
            reject("a { color: red; }", None, rejected("red"), (1, 12, 1, 15)),
            reject("a { color: rEd; }", None, rejected("rEd"), (1, 12, 1, 15)),
            reject("a { color: RED; }", None, rejected("RED"), (1, 12, 1, 15)),
            reject(
                "a { color: rebeccapurple; }",
                None,
                rejected("rebeccapurple"),
                (1, 12, 1, 25),
            ),
            reject(
                "a { background: #00c, red, #fff; }",
                None,
                rejected("red"),
                (1, 23, 1, 26),
            ),
            reject(
                "a { background: #fff1a1, rgb(250, 250, 0), black; }",
                None,
                rejected("black"),
                (1, 44, 1, 49),
            ),
            reject(
                "a { background:#cccccc,white,#12345a; }",
                None,
                rejected("white"),
                (1, 24, 1, 29),
            ),
            reject(
                "a { background: color(green); }",
                None,
                rejected("green"),
                (1, 23, 1, 28),
            ),
        ];

        Tester::new(NAME, json!(["never"]), pass, fail).test();
    }

    #[test]
    fn always_where_possible() {
        let pass = &[
            "a { color: black; }",
            "a { color: bLaCk; }",
            "a { color: BLACK; }",
            "a { color: rgb(0,0,1); }",
            "a { color: rgba(0,0,0,0.5); }",
            "a { color: rgb(0,0,0,50%); }",
            "a { color: color(black, a(50%)) }",
            "a { color: rgb(0, calc(0 + 0), 0, 0) }",
            "/** color: #000; */",
            "a::before { content: \"#000\" }",
            "a::before { content: '#000' }",
            "a { background-image: url(./thing.png#000); }",
            ".a { color: #fff0ffff; };",
            "a { color: rgba(0, 0, 0, 0); }",
        ];

        let fail = &[
            reject(
                "a { color: #000 }",
                None,
                expected("#000", "black"),
                (1, 12, 1, 16),
            ),
            reject(
                "a { color: #fFfFfF }",
                None,
                expected("#fFfFfF", "white"),
                (1, 12, 1, 19),
            ),
            reject(
                "a { color: #FFFFFF }",
                None,
                expected("#FFFFFF", "white"),
                (1, 12, 1, 19),
            ),
            reject(
                "a { background: #000f }",
                None,
                expected("#000f", "black"),
                (1, 17, 1, 22),
            ),
            reject(
                "a { color: #000000ff }",
                None,
                expected("#000000ff", "black"),
                (1, 12, 1, 21),
            ),
            reject(
                "a { color: rgb(0, 0, 0) }",
                None,
                expected("rgb(0,0,0)", "black"),
                (1, 12, 1, 24),
            ),
            reject(
                "a { color: rGb(0, 0, 0) }",
                None,
                expected("rGb(0,0,0)", "black"),
                (1, 12, 1, 24),
            ),
            reject(
                "a { color: RGB(0, 0, 0) }",
                None,
                expected("RGB(0,0,0)", "black"),
                (1, 12, 1, 24),
            ),
            reject(
                "a { color: rgba(0, 0, 0, 1) }",
                None,
                expected("rgba(0,0,0,1)", "black"),
                (1, 12, 1, 28),
            ),
            reject(
                "a { color: rgba(0 0 0 / 1) }",
                None,
                expected("rgba(0 0 0/1)", "black"),
                (1, 12, 1, 27),
            ),
            reject(
                "a { color: rgba(0, 0, 0, 100%) }",
                None,
                expected("rgba(0,0,0,100%)", "black"),
                (1, 12, 1, 31),
            ),
            reject(
                "a { color: rgb(0%,0%, 0%) }",
                None,
                expected("rgb(0%,0%,0%)", "black"),
                (1, 12, 1, 26),
            ),
            reject(
                "a { color: rgba(0%,0%, 0%  ,1) }",
                None,
                expected("rgba(0%,0%,0%,1)", "black"),
                (1, 12, 1, 31),
            ),
            reject(
                "a { color: rgba(0%,0%, 0%\n,100%) }",
                None,
                expected("rgba(0%,0%,0%,100%)", "black"),
                (1, 12, 2, 7),
            ),
            reject(
                "a { color: hsl(0,0%, 0%) }",
                None,
                expected("hsl(0,0%,0%)", "black"),
                (1, 12, 1, 25),
            ),
            reject(
                "a { color: hsla(0,0%, 0%  ,1) }",
                None,
                expected("hsla(0,0%,0%,1)", "black"),
                (1, 12, 1, 30),
            ),
            reject(
                "a { color: hsla(0,0%, 0%\n,100%) }",
                None,
                expected("hsla(0,0%,0%,100%)", "black"),
                (1, 12, 2, 7),
            ),
            reject(
                "a { color: hwb(0,0%, 0%) }",
                None,
                expected("hwb(0,0%,0%)", "red"),
                (1, 12, 1, 25),
            ),
            reject(
                "a { color: hwb(0,0%, 0%  ,1) }",
                None,
                expected("hwb(0,0%,0%,1)", "red"),
                (1, 12, 1, 29),
            ),
            reject(
                "a { color: hwb(0,0%, 0%\n,100%) }",
                None,
                expected("hwb(0,0%,0%,100%)", "red"),
                (1, 12, 2, 7),
            ),
            reject(
                "a { color: gray(0) }",
                None,
                expected("gray(0)", "black"),
                (1, 12, 1, 19),
            ),
            reject(
                "a { color: gray(0%) }",
                None,
                expected("gray(0%)", "black"),
                (1, 12, 1, 20),
            ),
            reject(
                "a { color: gray(0, 1) }",
                None,
                expected("gray(0,1)", "black"),
                (1, 12, 1, 22),
            ),
            reject(
                "a { color: gray(0, 100%) }",
                None,
                expected("gray(0,100%)", "black"),
                (1, 12, 1, 25),
            ),
            reject(
                "a { color: rgb(\n0 ,\n 0 ,\r\n 0) }",
                None,
                expected("rgb(0,0,0)", "black"),
                (1, 12, 4, 4),
            ),
            reject(
                "a { background: #302, rgb(\n0 ,\n 0 ,\r\n 0) }",
                None,
                expected("rgb(0,0,0)", "black"),
                (1, 23, 4, 4),
            ),
            reject(
                "a { color: color(#000 a(50%)) }",
                None,
                expected("#000", "black"),
                (1, 18, 1, 22),
            ),
        ];

        Tester::new(NAME, json!(["always-where-possible"]), pass, fail).test();
    }

    #[test]
    fn never_ignore_properties() {
        let pass = &[
            "a { composes: blue from 'lib/index.css'; }",
            "a { composes: grey blue from 'lib/index.css'; }",
            "a { my-property: red; }",
            "a { my-other-property: blue; }",
        ];

        let fail = &[
            reject(
                "a { color: rebeccapurple; }",
                None,
                rejected("rebeccapurple"),
                (1, 12, 1, 25),
            ),
            reject(
                "a { not-my-property: red; }",
                None,
                rejected("red"),
                (1, 22, 1, 25),
            ),
        ];

        Tester::new(
            NAME,
            json!(["never", { "ignoreProperties": ["composes", "/^my-/"] }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn never_ignore_properties_regex() {
        let pass = &["a { my-property: red; }"];
        let fail = &[reject(
            "a { color: rebeccapurple; }",
            None,
            rejected("rebeccapurple"),
            (1, 12, 1, 25),
        )];

        Tester::new(
            NAME,
            json!(["never", { "ignoreProperties": ["/^my-/"] }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn never_ignore_inside_function() {
        let pass = &[
            "a { color: #000; }",
            "a { color: gray(70%); }",
            "a { background-image: url(red); }",
            "a { color: map-get($color, blue(60%)); }",
            "a { color: map-get($color, blue); }",
            "a { background: color(blue); }",
        ];

        let fail = &[
            reject(
                "a { background: url(red) no-repeat rebeccapurple; }",
                None,
                rejected("rebeccapurple"),
                (1, 36, 1, 49),
            ),
            reject("a { color: red; }", None, rejected("red"), (1, 12, 1, 15)),
        ];

        Tester::new(
            NAME,
            json!(["never", { "ignore": ["inside-function"] }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn never_ignore_functions() {
        let pass = &[
            "a { color: #000; }",
            "a { color: var(--foo, red); }",
            "a { color: --foo(red); }",
        ];

        let fail = &[
            reject("a { color: red; }", None, rejected("red"), (1, 12, 1, 15)),
            reject(
                "a { color: red var(--foo, blue); }",
                None,
                rejected("red"),
                (1, 12, 1, 15),
            ),
        ];

        Tester::new(
            NAME,
            json!(["never", { "ignoreFunctions": ["var", "/^--/"] }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn always_where_possible_ignore_functions() {
        let pass = &[
            "a { color: black; }",
            "a { color: var(--foo, #000); }",
            "a { color: --foo(#000); }",
        ];

        let fail = &[reject(
            "a { color: #000; }",
            None,
            expected("#000", "black"),
            (1, 12, 1, 16),
        )];

        Tester::new(
            NAME,
            json!(["always-where-possible", { "ignoreFunctions": ["var", "/^--/"] }]),
            pass,
            fail,
        )
        .test();
    }
}

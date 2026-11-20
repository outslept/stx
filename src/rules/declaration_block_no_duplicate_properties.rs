use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

use crate::context::{Fix, LintContext};
use crate::rules::Rule;

#[derive(Debug)]
enum IgnoreMatcher {
    Exact(String),
    Regex(Regex),
}

impl IgnoreMatcher {
    fn from_str(s: &str) -> Option<Self> {
        if s.starts_with('/') && s.len() >= 2 {
            if let Some(last_slash) = s.rfind('/') {
                if last_slash > 0 {
                    let pattern = &s[1..last_slash];
                    let flags = &s[last_slash + 1..];
                    let mut builder = RegexBuilder::new(pattern);
                    if flags.contains('i') {
                        builder.case_insensitive(true);
                    }
                    return builder.build().ok().map(IgnoreMatcher::Regex);
                }
            }
        }
        Some(IgnoreMatcher::Exact(s.to_string()))
    }

    fn matches(&self, prop: &str) -> bool {
        match self {
            IgnoreMatcher::Exact(s) => s.eq_ignore_ascii_case(prop),
            IgnoreMatcher::Regex(re) => re.is_match(prop),
        }
    }
}

#[derive(Debug, Default)]
pub struct DeclarationBlockNoDuplicateProperties {
    ignore_duplicates: bool,
    ignore_diff_values: bool,
    ignore_diff_syntaxes: bool,
    ignore_prefixless_same_values: bool,
    ignore_properties: Vec<IgnoreMatcher>,
}

impl DeclarationBlockNoDuplicateProperties {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Option<Self> {
        if primary.as_bool() == Some(false) {
            return None;
        }

        let mut rule = Self::default();
        if let Some(sec) = secondary {
            rule.ignore_duplicates = has_option(sec, "ignore", "consecutive-duplicates");
            rule.ignore_diff_values = has_option(
                sec,
                "ignore",
                "consecutive-duplicates-with-different-values",
            );
            rule.ignore_diff_syntaxes = has_option(
                sec,
                "ignore",
                "consecutive-duplicates-with-different-syntaxes",
            );
            rule.ignore_prefixless_same_values = has_option(
                sec,
                "ignore",
                "consecutive-duplicates-with-same-prefixless-values",
            );

            if let Some(ig) = sec.get("ignoreProperties") {
                match ig {
                    Value::String(s) => {
                        if let Some(m) = IgnoreMatcher::from_str(s) {
                            rule.ignore_properties.push(m);
                        }
                    }
                    Value::Array(arr) => {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                if let Some(m) = IgnoreMatcher::from_str(s) {
                                    rule.ignore_properties.push(m);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Some(rule)
    }

    fn is_ignored_property(&self, prop: &str) -> bool {
        self.ignore_properties.iter().any(|m| m.matches(prop))
    }
}

fn has_option(sec: &Map<String, Value>, key: &str, needle: &str) -> bool {
    match sec.get(key) {
        Some(Value::String(s)) => s == needle,
        Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some(needle)),
        _ => false,
    }
}

pub fn rejected_message(property: &str) -> String {
    format!("Duplicate property \"{property}\"")
}

fn is_standard_syntax_property(prop: &str) -> bool {
    !prop.starts_with('$')
        && !prop.starts_with('@')
        && !prop.contains("#{")
        && !prop.contains("@{")
        && !prop.contains("$(")
}

fn is_standard_syntax_value(val: &str) -> bool {
    !val.starts_with('$')
        && !val.starts_with('@')
        && !val.contains('$')
        && !val.contains("#{")
        && !val.contains("@{")
}

fn vendor_unprefixed(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.first() == Some(&b'-') {
        let mut i = 1;
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        if i > 1 && i < bytes.len() && bytes[i] == b'-' {
            return &s[i + 1..];
        }
    }
    s
}

const COLOR_PROPERTIES: &[&str] = &[
    "color",
    "background-color",
    "border-color",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-block-color",
    "border-block-start-color",
    "border-block-end-color",
    "border-inline-color",
    "border-inline-start-color",
    "border-inline-end-color",
    "column-rule-color",
    "outline-color",
    "text-decoration-color",
    "text-emphasis-color",
    "caret-color",
    "accent-color",
    "scrollbar-color",
    "fill",
    "stroke",
    "stop-color",
    "flood-color",
    "lighting-color",
];

fn is_color_property(prop: &str) -> bool {
    COLOR_PROPERTIES
        .iter()
        .any(|p| p.eq_ignore_ascii_case(prop))
}

const NAMED_COLORS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "currentcolor",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "transparent",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

fn is_named_color(ident: &str) -> bool {
    NAMED_COLORS.iter().any(|c| c.eq_ignore_ascii_case(ident))
}

#[derive(Debug, Clone)]
enum CssValNode {
    Identifier(String),
    Dimension(String),
    Percentage,
    Number,
    StringLit,
    Hash,
    Url,
    Operator(char),
    Function(String, Vec<CssValNode>),
    Parentheses(Vec<CssValNode>),
}

fn skip_parens(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == b'"' || b == b'\'' {
            let quote = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
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
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                i += 1;
                break;
            }
        }
        i += 1;
    }
    i.min(len)
}

fn parse_css_value_nodes(s: &str) -> Option<Vec<CssValNode>> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut out = Vec::new();

    while i < len {
        let b = bytes[i];

        if b.is_ascii_whitespace() {
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

        if b == b'"' || b == b'\'' {
            let quote = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(CssValNode::StringLit);
            continue;
        }

        if b == b'#' {
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push(CssValNode::Hash);
            continue;
        }

        if b == b'(' {
            let close = skip_parens(bytes, i);
            let inner_end = if close > i + 1 && bytes[close - 1] == b')' {
                close - 1
            } else {
                return None;
            };
            let children = parse_css_value_nodes(&s[i + 1..inner_end])?;
            out.push(CssValNode::Parentheses(children));
            i = close;
            continue;
        }

        if b == b'+' || b == b'*' || b == b'/' || b == b',' {
            out.push(CssValNode::Operator(b as char));
            i += 1;
            continue;
        }

        // Number, Dimension, or Percentage
        if b.is_ascii_digit()
            || (b == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
            || ((b == b'-' || b == b'+')
                && i + 1 < len
                && (bytes[i + 1].is_ascii_digit()
                    || (bytes[i + 1] == b'.' && i + 2 < len && bytes[i + 2].is_ascii_digit())))
        {
            if b == b'-' || b == b'+' {
                i += 1;
            }
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && bytes[i] == b'.' {
                i += 1;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < len && bytes[i] == b'%' {
                i += 1;
                out.push(CssValNode::Percentage);
            } else if i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'-') {
                let unit_start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
                    i += 1;
                }
                out.push(CssValNode::Dimension(s[unit_start..i].to_string()));
            } else {
                out.push(CssValNode::Number);
            }
            continue;
        }

        if b == b'-' && (i + 1 >= len || bytes[i + 1].is_ascii_whitespace()) {
            out.push(CssValNode::Operator('-'));
            i += 1;
            continue;
        }

        // Identifier or Function
        let start = i;
        while i < len {
            let c = bytes[i];
            if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || !c.is_ascii() {
                i += 1;
            } else {
                break;
            }
        }

        if i == start {
            return None;
        }

        let name = &s[start..i];
        if i < len && bytes[i] == b'(' {
            let close = skip_parens(bytes, i);
            let inner_end = if close > i + 1 && bytes[close - 1] == b')' {
                close - 1
            } else {
                return None;
            };
            if name.eq_ignore_ascii_case("url") {
                out.push(CssValNode::Url);
            } else {
                let children = parse_css_value_nodes(&s[i + 1..inner_end])?;
                out.push(CssValNode::Function(name.to_string(), children));
            }
            i = close;
        } else {
            out.push(CssValNode::Identifier(name.to_string()));
        }
    }

    Some(out)
}

fn is_equal_value_nodes(nodes1: &[CssValNode], nodes2: &[CssValNode], property: &str) -> bool {
    if nodes1.len() != nodes2.len() {
        return false;
    }
    let is_color_prop = is_color_property(property);

    for (n1, n2) in nodes1.iter().zip(nodes2.iter()) {
        match (n1, n2) {
            (CssValNode::Identifier(name1), CssValNode::Identifier(name2)) => {
                if name1.starts_with("--") && name2.starts_with("--") {
                    continue;
                }
                if is_color_prop && is_named_color(name1) && is_named_color(name2) {
                    continue;
                }
                if !name1.eq_ignore_ascii_case(name2) {
                    return false;
                }
            }
            (CssValNode::Dimension(u1), CssValNode::Dimension(u2)) => {
                if u1 != u2 {
                    return false;
                }
            }
            (CssValNode::Percentage, CssValNode::Percentage)
            | (CssValNode::Number, CssValNode::Number)
            | (CssValNode::StringLit, CssValNode::StringLit)
            | (CssValNode::Hash, CssValNode::Hash)
            | (CssValNode::Url, CssValNode::Url) => {}
            (CssValNode::Operator(op1), CssValNode::Operator(op2)) => {
                if op1 != op2 {
                    return false;
                }
            }
            (CssValNode::Function(f1, c1), CssValNode::Function(f2, c2)) => {
                if !f1.eq_ignore_ascii_case(f2) || !is_equal_value_nodes(c1, c2, property) {
                    return false;
                }
            }
            (CssValNode::Parentheses(c1), CssValNode::Parentheses(c2)) => {
                if !is_equal_value_nodes(c1, c2, property) {
                    return false;
                }
            }
            _ => return false,
        }
    }

    true
}

fn is_equal_value_syntaxes(value1: &str, value2: &str, property: &str) -> bool {
    if value1 == value2 {
        return true;
    }
    if !(is_standard_syntax_value(value1) && is_standard_syntax_value(value2)) {
        return false;
    }
    let (Some(nodes1), Some(nodes2)) =
        (parse_css_value_nodes(value1), parse_css_value_nodes(value2))
    else {
        return false;
    };
    is_equal_value_nodes(&nodes1, &nodes2, property)
}

#[derive(Debug, Clone)]
struct DeclInfo<'a> {
    prop: &'a str,
    lower_prop: String,
    value: &'a str,
    important: bool,
    prop_start: usize,
    raw_before_start: usize,
    stmt_end: usize,
}

#[derive(Debug)]
struct PendingReport<'a> {
    decl: DeclInfo<'a>,
}

#[derive(Debug)]
struct BlockState<'a> {
    last_token_end: usize,
    active_decls: Vec<DeclInfo<'a>>,
    previous_considered_prop: String,
    reports: Vec<PendingReport<'a>>,
}

impl<'a> BlockState<'a> {
    fn new(open_brace_pos: usize) -> Self {
        Self {
            last_token_end: open_brace_pos + 1,
            active_decls: Vec::new(),
            previous_considered_prop: String::new(),
            reports: Vec::new(),
        }
    }
}

fn split_value_and_important(raw_val: &str) -> (&str, bool) {
    let trimmed = raw_val.trim();
    if let Some(bang_pos) = trimmed.rfind('!') {
        let after_bang = trimmed[bang_pos + 1..].trim();
        if after_bang.eq_ignore_ascii_case("important") {
            return (trimmed[..bang_pos].trim(), true);
        }
    }
    (trimmed, false)
}

fn skip_braces(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                i += 1;
                break;
            }
        }
        i += 1;
    }
    i.min(len)
}

impl DeclarationBlockNoDuplicateProperties {
    fn process_declaration<'a>(&self, block: &mut BlockState<'a>, decl: DeclInfo<'a>) {
        let prop = decl.prop;
        if !is_standard_syntax_property(prop) {
            return;
        }
        if prop.starts_with("--") {
            return;
        }
        if self.is_ignored_property(prop) {
            return;
        }
        if decl.lower_prop == "src" {
            return;
        }

        let index_duplicate = block
            .active_decls
            .iter()
            .position(|d| d.lower_prop == decl.lower_prop);
        let duplicates_are_consecutive = block.previous_considered_prop == decl.lower_prop;
        block.previous_considered_prop = decl.lower_prop.clone();

        let Some(idx) = index_duplicate else {
            block.active_decls.push(decl);
            return;
        };

        let duplicate_decl = block.active_decls[idx].clone();
        let duplicate_is_more_important = !decl.important && duplicate_decl.important;
        let unprefixed_duplicates_are_equal =
            vendor_unprefixed(decl.value) == vendor_unprefixed(duplicate_decl.value);

        let mut fix_or_report = |block: &mut BlockState<'a>| {
            let target = if duplicate_is_more_important {
                decl.clone()
            } else {
                block.active_decls[idx] = decl.clone();
                duplicate_decl.clone()
            };
            block.reports.push(PendingReport { decl: target });
        };

        if self.ignore_diff_values
            || self.ignore_diff_syntaxes
            || self.ignore_prefixless_same_values
        {
            if !duplicates_are_consecutive
                || (self.ignore_prefixless_same_values && !unprefixed_duplicates_are_equal)
            {
                fix_or_report(block);
                return;
            }
            if self.ignore_diff_syntaxes {
                if is_equal_value_syntaxes(decl.value, duplicate_decl.value, prop) {
                    fix_or_report(block);
                    return;
                }
            }
            if decl.value != duplicate_decl.value {
                return;
            }
            block.active_decls[idx] = decl;
            block.reports.push(PendingReport {
                decl: duplicate_decl,
            });
            return;
        }

        if self.ignore_duplicates && duplicates_are_consecutive {
            return;
        }

        fix_or_report(block);
    }

    fn flush_block_reports<'a>(&self, block: BlockState<'a>, ctx: &mut LintContext<'a>) {
        if block.reports.is_empty() {
            return;
        }
        let bytes = ctx.source.as_bytes();
        let n = block.reports.len();
        let mut prev_del_end = 0usize;

        for (idx, report) in block.reports.iter().enumerate() {
            let d = &report.decl;
            let mut del_start = d.raw_before_start.max(prev_del_end);
            let mut del_end = d.stmt_end + (del_start - d.raw_before_start);

            let max_end = if idx + 1 < n {
                let next_d = &block.reports[idx + 1].decl;
                if next_d.raw_before_start >= d.stmt_end {
                    next_d.raw_before_start + 1
                } else {
                    bytes.len()
                }
            } else {
                bytes.len()
            };

            while del_end < max_end && del_end < bytes.len() && bytes[del_start] == bytes[del_end] {
                del_start += 1;
                del_end += 1;
            }

            prev_del_end = del_end;

            let prop_span = Span {
                start: d.prop_start as u32,
                end: (d.prop_start + d.prop.len()) as u32,
            };
            let fix = Fix::single(
                Span {
                    start: del_start as u32,
                    end: del_end as u32,
                },
                String::new(),
            );

            ctx.report_with_fix(self.name(), prop_span, rejected_message(d.prop), Some(fix));
        }
    }
}

impl Rule for DeclarationBlockNoDuplicateProperties {
    fn name(&self) -> &'static str {
        "declaration-block-no-duplicate-properties"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let source = ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();

        let mut stack: Vec<BlockState<'a>> = Vec::new();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            if b == b'{' {
                stack.push(BlockState::new(i));
                i += 1;
                continue;
            }

            if b == b'}' {
                if let Some(popped) = stack.pop() {
                    self.flush_block_reports(popped, ctx);
                }
                if let Some(parent) = stack.last_mut() {
                    parent.last_token_end = i + 1;
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
                if let Some(current) = stack.last_mut() {
                    current.last_token_end = i;
                }
                continue;
            }

            let stmt_start = i;
            let mut paren_depth = 0usize;
            let mut first_colon: Option<usize> = None;

            while i < len {
                let c = bytes[i];
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == b'"' || c == b'\'' {
                    let quote = c;
                    i += 1;
                    while i < len {
                        if bytes[i] == b'\\' {
                            i += 2;
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
                if c == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                    i += 2;
                    while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                        i += 1;
                    }
                    i = (i + 2).min(len);
                    continue;
                }
                if c == b'{' && i > stmt_start && (bytes[i - 1] == b'#' || bytes[i - 1] == b'@') {
                    i = skip_braces(bytes, i);
                    continue;
                }
                if c == b'(' {
                    paren_depth += 1;
                } else if c == b')' {
                    paren_depth = paren_depth.saturating_sub(1);
                } else if paren_depth == 0 {
                    if c == b':' && first_colon.is_none() {
                        first_colon = Some(i);
                    } else if c == b'{' || c == b';' || c == b'}' {
                        break;
                    }
                }
                i += 1;
            }

            if i < len && bytes[i] == b'{' {
                stack.push(BlockState::new(i));
                i += 1;
            } else {
                let stmt_end = if i < len && bytes[i] == b';' {
                    i + 1
                } else {
                    i
                };
                let stmt_slice = &source[stmt_start..i];
                let is_less_at_var = stmt_slice.starts_with('@') && first_colon.is_some();

                if !stmt_slice.starts_with('@') || is_less_at_var {
                    if let Some(colon_pos) = first_colon {
                        let prop = source[stmt_start..colon_pos].trim();
                        let raw_val = &source[colon_pos + 1..i];
                        let (value, important) = split_value_and_important(raw_val);

                        if let Some(current) = stack.last_mut() {
                            let decl_info = DeclInfo {
                                prop,
                                lower_prop: prop.to_ascii_lowercase(),
                                value,
                                important,
                                prop_start: stmt_start,
                                raw_before_start: current.last_token_end,
                                stmt_end,
                            };
                            self.process_declaration(current, decl_info);
                        }
                    }
                }

                if let Some(current) = stack.last_mut() {
                    current.last_token_end = stmt_end;
                }
                i = stmt_end;
            }
        }

        while let Some(popped) = stack.pop() {
            self.flush_block_reports(popped, ctx);
        }
    }
}

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
        if s.starts_with('/')
            && s.len() >= 2
            && let Some(last_slash) = s.rfind('/')
            && last_slash > 0
        {
            let pattern = &s[1..last_slash];
            let flags = &s[last_slash + 1..];
            let mut builder = RegexBuilder::new(pattern);
            if flags.contains('i') {
                builder.case_insensitive(true);
            }
            return builder.build().ok().map(IgnoreMatcher::Regex);
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
                            if let Some(s) = item.as_str()
                                && let Some(m) = IgnoreMatcher::from_str(s)
                            {
                                rule.ignore_properties.push(m);
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

        let fix_or_report = |block: &mut BlockState<'a>| {
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
            if self.ignore_diff_syntaxes
                && is_equal_value_syntaxes(decl.value, duplicate_decl.value, prop)
            {
                fix_or_report(block);
                return;
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

                if (!stmt_slice.starts_with('@') || is_less_at_var)
                    && let Some(colon_pos) = first_colon
                {
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

#[cfg(test)]
mod tests {
    use crate::tester::{
        Tester, reject, reject_edit, reject_edit_warnings, reject_message, reject_warnings,
    };
    use serde_json::json;

    const RULE_NAME: &str = "declaration-block-no-duplicate-properties";

    fn rejected(property: &str) -> String {
        format!("Duplicate property \"{property}\"")
    }

    #[test]
    fn declaration_block_no_duplicate_properties_default() {
        let pass = [
            "a { color: pink; }",
            "a { color: pink; background: orange; }",
            "a { color: pink; { &:hover { color: orange; } } }",
            "a { color: pink; @media { color: orange; } }",
            "a { color: pink; @media { color: orange; &::before { color: black; } } }",
            "a { $scss: 0; $scss: $scss + 1; }",
            "a { @less: 0; @less: @less + 1; }",
            "a { --custom-property: 0; --custom-property: 1; }",
            "@fontface { src: url(font.eof); src: url(font.woff) }",
            "@fontface { sRc: url(font.eof); sRc: url(font.woff) }",
            "@fontface { SRC: url(font.eof); SRC: url(font.woff) }",
        ];
        let fail = [
            reject(
                "a { color: pink; color: orange }",
                "a { color: orange }",
                rejected("color"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { cOlOr: pink; color: orange }",
                "a { color: orange }",
                rejected("cOlOr"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { color: pink; cOlOr: orange }",
                "a { cOlOr: orange }",
                rejected("color"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { cOlOr: pink; cOlOr: orange }",
                "a { cOlOr: orange }",
                rejected("cOlOr"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { COLOR: pink; color: orange }",
                "a { color: orange }",
                rejected("COLOR"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { color: pink; COLOR: orange }",
                "a { COLOR: orange }",
                rejected("color"),
                (1, 5, 1, 10),
            ),
            reject_warnings(
                "a { COlOr: pink; coLOR: pink; color: pink }",
                "a { color: pink }",
                vec![
                    (rejected("COlOr"), (1, 5, 1, 10)),
                    (rejected("coLOR"), (1, 18, 1, 23)),
                ],
            ),
            reject_warnings(
                "a { color: pink; color: pink; color: orange }",
                "a { color: orange }",
                vec![
                    (rejected("color"), (1, 5, 1, 10)),
                    (rejected("color"), (1, 18, 1, 23)),
                ],
            ),
            reject(
                "a { color: pink; background: orange; color: orange }",
                "a { background: orange; color: orange }",
                rejected("color"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { color: pink; background: orange; background: pink; }",
                "a { color: pink; background: pink; }",
                rejected("background"),
                (1, 18, 1, 28),
            ),
            reject(
                "a { color: pink; { &:hover { color: orange; color: black; } } }",
                "a { color: pink; { &:hover { color: black; } } }",
                rejected("color"),
                (1, 30, 1, 35),
            ),
            reject(
                "a { color: pink; @media { color: orange; color: black; } }",
                "a { color: pink; @media { color: black; } }",
                rejected("color"),
                (1, 27, 1, 32),
            ),
            reject(
                "@media { color: orange; .foo { color: black; color: white; } }",
                "@media { color: orange; .foo { color: white; } }",
                rejected("color"),
                (1, 32, 1, 37),
            ),
            reject(
                "a { color: pink; @media { color: orange; &::before { color: black; color: white; } } }",
                "a { color: pink; @media { color: orange; &::before { color: white; } } }",
                rejected("color"),
                (1, 54, 1, 59),
            ),
            reject(
                "a { color: pink; @media { color: orange; .foo { color: black; color: white; } } }",
                "a { color: pink; @media { color: orange; .foo { color: white; } } }",
                rejected("color"),
                (1, 49, 1, 54),
            ),
            reject(
                "a { -webkit-border-radius: 12px; -webkit-border-radius: 10px; }",
                "a { -webkit-border-radius: 10px; }",
                rejected("-webkit-border-radius"),
                (1, 5, 1, 26),
            ),
            reject(
                "a { -WEBKIT-border-radius: 12px; -webkit-BORDER-radius: 10px; }",
                "a { -webkit-BORDER-radius: 10px; }",
                rejected("-WEBKIT-border-radius"),
                (1, 5, 1, 26),
            ),
            reject(
                "a { color: red !important; color: blue; }",
                "a { color: red !important; }",
                rejected("color"),
                (1, 28, 1, 33),
            ),
            reject(
                "a { color: red !important; color: blue !important; }",
                "a { color: blue !important; }",
                rejected("color"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { scrollbar-color: rebeccapurple green; scrollbar-color: rebeccapurple transparent; }",
                "a { scrollbar-color: rebeccapurple transparent; }",
                rejected("scrollbar-color"),
                (1, 5, 1, 20),
            ),
            reject(
                "p { border-color: yellow green #000 red; border-color: black; }",
                "p { border-color: black; }",
                rejected("border-color"),
                (1, 5, 1, 17),
            ),
            reject(
                "p { animation-name: red; animation-name: black; }",
                "p { animation-name: black; }",
                rejected("animation-name"),
                (1, 5, 1, 19),
            ),
            reject(
                "a { border-color: red yellow; border-color: green; }",
                "a { border-color: green; }",
                rejected("border-color"),
                (1, 5, 1, 17),
            ),
            reject(
                "a { scrollbar-color: red #000; scrollbar-color: blue; }",
                "a { scrollbar-color: blue; }",
                rejected("scrollbar-color"),
                (1, 5, 1, 20),
            ),
            reject(
                "a { border-color: red #000; border-color: blue green; }",
                "a { border-color: blue green; }",
                rejected("border-color"),
                (1, 5, 1, 17),
            ),
        ];

        Tester::new(RULE_NAME, json!([true]), pass, fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_edit_info() {
        let fail = [
            reject_edit(
                "a { color: pink; color: orange }",
                rejected("color"),
                (1, 5, 1, 10),
                (11, 24, ""),
            ),
            reject_edit_warnings(
                "a { COlOr: pink; coLOR: pink; color: pink }",
                vec![
                    (rejected("COlOr"), (1, 5, 1, 10), Some((4, 17, ""))),
                    (rejected("coLOR"), (1, 18, 1, 23), Some((19, 32, ""))),
                ],
            ),
            reject_edit(
                "a { color: pink; background: orange; color: orange }",
                rejected("color"),
                (1, 5, 1, 10),
                (4, 17, ""),
            ),
            reject_edit(
                "a { color: pink; background: orange; background: pink; }",
                rejected("background"),
                (1, 18, 1, 28),
                (29, 49, ""),
            ),
        ];

        Tester::new(RULE_NAME, json!([true]), [], fail)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_consecutive() {
        let pass = [
            "p { font-size: 16px; font-size: 1rem; }",
            "p { display: inline-block; font-size: 16px; font-size: 1rem; }",
            "p { font-size: 16px; font-size: 1rem; color: red; }",
            "p { display: inline-block; font-size: 16px; font-size: 1rem; color: red; }",
        ];
        let fail = [
            reject_message(
                "p { font-size: 16px; font-weight: 400; font-size: 1rem; }",
                "p { font-weight: 400; font-size: 1rem; }",
                rejected("font-size"),
            ),
            reject_message(
                "p { display: inline-block; font-size: 16px; font-weight: 400; font-size: 1rem; }",
                "p { display: inline-block; font-weight: 400; font-size: 1rem; }",
                rejected("font-size"),
            ),
            reject_message(
                "p { font-size: 16px; font-weight: 400; font-size: 1rem; color: red; }",
                "p { font-weight: 400; font-size: 1rem; color: red; }",
                rejected("font-size"),
            ),
            reject_message(
                "p { display: inline-block; font-size: 16px; font-weight: 400; font-size: 1rem; color: red; }",
                "p { display: inline-block; font-weight: 400; font-size: 1rem; color: red; }",
                rejected("font-size"),
            ),
            reject_message(
                "p { font-size: 16px !important; font-weight: 400; font-size: 1rem; }",
                "p { font-size: 16px !important; font-weight: 400; }",
                rejected("font-size"),
            ),
            reject_warnings(
                "a { font-size: 16px; color: red; font-size: 1rem; color: blue; }",
                "a { font-size: 1rem; color: blue; }",
                vec![
                    (rejected("font-size"), (1, 5, 1, 14)),
                    (rejected("color"), (1, 22, 1, 27)),
                ],
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignore": ["consecutive-duplicates"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_consecutive_edit_info() {
        let fail = [
            reject_message(
                "p { font-size: 16px; font-weight: 400; font-size: 1rem; }",
                None,
                rejected("font-size"),
            )
            .with_edit_info(0, (9, 26, "")),
            reject_message(
                "p { font-size: 16px !important; font-weight: 400; font-size: 1rem; }",
                None,
                rejected("font-size"),
            )
            .with_edit_info(0, (50, 67, "")),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignore": ["consecutive-duplicates"] }]),
            [],
            fail,
        )
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_diff_values() {
        let pass = [
            "p { font-size: 16px; font-size: 1rem; font-weight: 400; }",
            "p { font-size: 16px; font-size: 18px; font-weight: 400; }",
        ];
        let fail = [
            reject_message(
                "p { font-size: 16px; font-weight: 400; font-size: 1rem; }",
                "p { font-weight: 400; font-size: 1rem; }",
                rejected("font-size"),
            ),
            reject_message(
                "p { font-size: 16px; font-size: 16px; font-weight: 400; }",
                "p { font-size: 16px; font-weight: 400; }",
                rejected("font-size"),
            ),
            reject(
                "p { font-size: 16px !important; font-weight: 400; font-size: 1rem; }",
                "p { font-size: 16px !important; font-weight: 400; }",
                rejected("font-size"),
                (1, 51, 1, 60),
            ),
            reject(
                "p { font-size: 16px; font-size: 16px !important; font-weight: 400; }",
                "p { font-size: 16px !important; font-weight: 400; }",
                rejected("font-size"),
                (1, 5, 1, 14),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignore": ["consecutive-duplicates-with-different-values"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_diff_syntaxes() {
        let pass = [
            "p { width: 100vw; height: 100vh; }",
            "p { width: 100vw; width: 100dvw; height: 100vh; }",
            "p { margin: 10dvw 10dvw; margin: 10vw 10vw; padding: 0; }",
            "p { margin: 10dvh 10dvw 10dvh; margin: 10vw 10vw; padding: 0; }",
            "p { width: 100%; width: fit-content; }",
            "p { width: min-content; width: max-content; }",
            "p { width: calc(10px + 2px); width: calc(10px + 2rem); }",
            "p { width: calc(10px + 2px); width: calc(10rem + 2rem); }",
            "p { width: min(10px, 11px); width: max(10px, 11px); }",
            "p { width: calc((10px + 2px)); width: calc((10rem + 2rem)); }",
            "p { width: calc((10px + 2px) + 10px); width: calc((10rem + 2rem) + 10px); }",
            "p { width: $a; width: calc(1 + $a); }",
            "p { width: _$a; width: _$a2; }",
            "p { width: env(foo); width: env(--bar); }",
            "p { width: env(--foo); width: env(bar); }",
        ];
        let fail = [
            reject_message(
                "p { width: 100vw ; width: 100vw; }",
                "p { width: 100vw; }",
                rejected("width"),
            ),
            reject_message(
                "p { width: calc(100vw /* a comment */  + 10vw); width: calc(100vw + 10vw); }",
                "p { width: calc(100vw + 10vw); }",
                rejected("width"),
            ),
            reject_message(
                "p { width: 100vw; height: 100vh; width: 100dvw; }",
                "p { height: 100vh; width: 100dvw; }",
                rejected("width"),
            ),
            reject_message(
                "p { width: 100vw; width: 100vw; height: 100vh; }",
                "p { width: 100vw; height: 100vh; }",
                rejected("width"),
            ),
            reject_message(
                "p { margin: 10vw 10vw; margin: 10vw 10vw; padding: 0; }",
                "p { margin: 10vw 10vw; padding: 0; }",
                rejected("margin"),
            ),
            reject_message(
                "p { width: 100vw; width: 50vw; height: 100vh; }",
                "p { width: 50vw; height: 100vh; }",
                rejected("width"),
            ),
            reject_message(
                "p { width: 100vw !important; height: 100vh; width: 100dvw; }",
                "p { width: 100vw !important; height: 100vh; }",
                rejected("width"),
            ),
            reject_message(
                "p { width: 100vw; height: 100vh; width: 100dvw !important; }",
                "p { height: 100vh; width: 100dvw !important; }",
                rejected("width"),
            ),
            reject_message(
                "p { width: calc(10px + 4rem); width: calc(10px + 2rem); }",
                "p { width: calc(10px + 2rem); }",
                rejected("width"),
            ),
            reject_message(
                "p { width: CaLC(10px + 4rem); width: calc(10px + 2rem); }",
                "p { width: calc(10px + 2rem); }",
                rejected("width"),
            ),
            reject_message(
                "p { width: var(--foo); width: var(--bar); }",
                "p { width: var(--bar); }",
                rejected("width"),
            ),
            reject_message(
                "p { content: \"foo\"; content: \"bar\"; }",
                "p { content: \"bar\"; }",
                rejected("content"),
            ),
            reject_message(
                "p { image: url(#foo); image: url(#bar); }",
                "p { image: url(#bar); }",
                rejected("image"),
            ),
            reject_message(
                "p { color: red; color: blue; }",
                "p { color: blue; }",
                rejected("color"),
            ),
            reject_message(
                "p { color: red; color: green; }",
                "p { color: green; }",
                rejected("color"),
            ),
            reject_message(
                "p { background-color: red; background-color: blue; }",
                "p { background-color: blue; }",
                rejected("background-color"),
            ),
            reject_message(
                "p { color: red; background: orange; color: blue; }",
                "p { background: orange; color: blue; }",
                rejected("color"),
            ),
            reject_message(
                "a { scrollbar-color: red blue; scrollbar-color: green yellow; }",
                "a { scrollbar-color: green yellow; }",
                rejected("scrollbar-color"),
            ),
            reject_message(
                "a { border-color: red green blue yellow; border-color: black white gray silver; }",
                "a { border-color: black white gray silver; }",
                rejected("border-color"),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignore": ["consecutive-duplicates-with-different-syntaxes"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_prefixless() {
        let pass = [
            "p { width: -moz-fit-content; width: fit-content; }",
            "p { width: fit-content; width: -moz-fit-content; }",
            "p { width: -MOZ-fit-content; width: fit-content; }",
            "p { width: -webkit-fit-content; width: -moz-fit-content; width: fit-content; }",
        ];
        let fail = [
            reject_message(
                "p { width: fit-content; height: 32px; width: -moz-fit-content; }",
                "p { height: 32px; width: -moz-fit-content; }",
                rejected("width"),
            ),
            reject_message(
                "p { width: 100%; width: -moz-fit-content; height: 32px; }",
                "p { width: -moz-fit-content; height: 32px; }",
                rejected("width"),
            ),
            reject_message(
                "p { width: -moz-fit-content; width: -moz-fit-content; }",
                "p { width: -moz-fit-content; }",
                rejected("width"),
            ),
            reject(
                "p { width: -moz-fit-content; width: -moz-fit-content !important; }",
                "p { width: -moz-fit-content !important; }",
                rejected("width"),
                (1, 5, 1, 10),
            ),
            reject(
                "p { width: 100% !important; width: -moz-fit-content; }",
                "p { width: 100% !important; }",
                rejected("width"),
                (1, 29, 1, 34),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignore": ["consecutive-duplicates-with-same-prefixless-values"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_diff_values_edit_info() {
        let fail = [reject_edit(
            "p { font-size: 16px; font-size: 16px !important; font-weight: 400; }",
            rejected("font-size"),
            (1, 5, 1, 14),
            (19, 36, ""),
        )];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignore": ["consecutive-duplicates-with-different-values"] }]),
            [],
            fail,
        )
        .with_compute_edit_info(true)
        .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_properties_string() {
        let pass = [
            "p { color: pink; color: orange; }",
            "p { COLOR: pink; color: orange; }",
            "p { color: pink; background: orange; color: orange }",
            "p { color: pink; color: orange !important; }",
        ];
        let fail = [
            reject_message(
                "p { color: pink; background: orange; background: white; }",
                "p { color: pink; background: white; }",
                rejected("background"),
            ),
            reject_message(
                "p { background: orange; color: pink; background: white; }",
                "p { color: pink; background: white; }",
                rejected("background"),
            ),
            reject_message(
                "p { background: orange; color: pink; background: white !important; }",
                "p { color: pink; background: white !important; }",
                rejected("background"),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignoreProperties": ["color"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn declaration_block_no_duplicate_properties_ignore_properties_regex() {
        let pass = [
            "p { background-color: pink; background-color: orange; }",
            "p { background-color: pink; color: orange; background-color: orange }",
            "p { padding-left: auto; padding-left: 1rem; }",
        ];
        let fail = [
            reject_message(
                "p { color: pink; background: orange; background: white; }",
                "p { color: pink; background: white; }",
                rejected("background"),
            ),
            reject_message(
                "p { background: orange; color: pink; background: white; }",
                "p { color: pink; background: white; }",
                rejected("background"),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignoreProperties": ["/background-/", "/padding-/"] }]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    #[ignore = "postcss-html not supported"]
    fn declaration_block_no_duplicate_properties_html() {
        Tester::new(
            RULE_NAME,
            json!([true]),
            ["<style>a { color: pink; }</style>"],
            [],
        )
        .with_custom_syntax("postcss-html")
        .with_fix(true)
        .test();
    }

    #[test]
    #[ignore = "naive-css-in-js not supported"]
    fn declaration_block_no_duplicate_properties_css_in_js() {
        Tester::new(RULE_NAME, json!([true]), ["css` color: pink; `;"], [])
            .with_custom_syntax("naive-css-in-js")
            .test();
    }
}

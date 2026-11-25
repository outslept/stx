//! Rule configuration parsing, regular expression compilation, and Stylelint
//! option matching utilities.

use regex::Regex;
use serde_json::{Map, Value};

/// Default kebab-case regular expression used by `*-pattern` rules in Stylelint.
pub const DEFAULT_KEBAB_PATTERN: &str = "^([a-z][a-z0-9]*)(-[a-z0-9]+)*$";

/// Returns `true` if `s` is a valid `kebab-case` identifier (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`).
pub fn is_kebab_case(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    let mut prev_hyphen = false;
    for &b in &bytes[1..] {
        if b == b'-' {
            if prev_hyphen {
                return false;
            }
            prev_hyphen = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }
    !prev_hyphen
}

#[derive(Debug, Clone)]
enum PatternKind {
    DefaultKebab,
    Custom(Regex),
}

/// Compiled pattern matcher used by `*-pattern` rules.
#[derive(Debug, Clone)]
pub struct PatternMatcher {
    /// Original pattern string used in diagnostic messages.
    pub raw_pattern: String,
    kind: PatternKind,
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self {
            raw_pattern: DEFAULT_KEBAB_PATTERN.to_string(),
            kind: PatternKind::DefaultKebab,
        }
    }
}

impl PatternMatcher {
    /// Compiles a pattern option (either a raw regex string or a `/regex/flags` literal),
    /// falling back to [`DEFAULT_KEBAB_PATTERN`] if `None`.
    pub fn new<S: AsRef<str>>(primary_str: Option<S>) -> Self {
        let Some(s_ref) = primary_str else {
            return Self::default();
        };
        let s = s_ref.as_ref();
        if s == DEFAULT_KEBAB_PATTERN {
            return Self::default();
        }
        if let Some(regex) = compile_js_regex_or_plain(s) {
            return Self {
                raw_pattern: s.to_string(),
                kind: PatternKind::Custom(regex),
            };
        }
        Self {
            raw_pattern: s.to_string(),
            kind: PatternKind::DefaultKebab,
        }
    }

    /// Returns `true` if `s` matches the configured pattern.
    #[inline]
    pub fn is_match(&self, s: &str) -> bool {
        match &self.kind {
            PatternKind::DefaultKebab => is_kebab_case(s),
            PatternKind::Custom(re) => re.is_match(s),
        }
    }
}

/// A single matcher entry in a Stylelint option array or scalar (either an exact
/// string or a compiled regular expression).
#[derive(Debug, Clone)]
pub enum OptionMatcher {
    /// Exact literal string comparison.
    Exact(String),
    /// Compiled regular expression (from `/pattern/` or `/pattern/i` syntax).
    Regex(Regex),
}

impl OptionMatcher {
    /// Parses a single string into either an [`OptionMatcher::Regex`] (if wrapped in `/.../`)
    /// or an [`OptionMatcher::Exact`] matcher.
    pub fn parse(raw: &str) -> Self {
        if let Some(re) = parse_slash_regex(raw) {
            Self::Regex(re)
        } else {
            Self::Exact(raw.to_string())
        }
    }

    /// Returns `true` if `s` matches this entry (case-sensitive for exact strings).
    #[inline]
    pub fn is_match(&self, s: &str) -> bool {
        match self {
            Self::Exact(ex) => ex == s,
            Self::Regex(re) => re.is_match(s),
        }
    }

    /// Returns `true` if `s` matches this entry, using ASCII case-insensitive comparison
    /// for [`OptionMatcher::Exact`] entries.
    #[inline]
    pub fn is_match_ignore_ascii_case(&self, s: &str) -> bool {
        match self {
            Self::Exact(ex) => ex.eq_ignore_ascii_case(s),
            Self::Regex(re) => re.is_match(s),
        }
    }
}

/// Extracts a list of [`OptionMatcher`]s from `secondary[key]`, supporting both
/// a single string value and an array of strings.
pub fn parse_option_matchers(
    secondary: Option<&Map<String, Value>>,
    key: &str,
) -> Vec<OptionMatcher> {
    let Some(val) = secondary.and_then(|m| m.get(key)) else {
        return Vec::new();
    };
    parse_primary_option_matchers(val)
}

/// Parses a primary or secondary [`Value`] (either a single string or an array of strings)
/// into a vector of [`OptionMatcher`]s.
pub fn parse_primary_option_matchers(val: &Value) -> Vec<OptionMatcher> {
    match val {
        Value::String(s) => vec![OptionMatcher::parse(s)],
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(OptionMatcher::parse)
            .collect(),
        _ => Vec::new(),
    }
}

/// Returns `true` if `input` matches any entry in `matchers` (case-sensitive for literal strings).
#[inline]
pub fn options_matches(matchers: &[OptionMatcher], input: &str) -> bool {
    matchers.iter().any(|m| m.is_match(input))
}

/// Returns `true` if `input` matches any entry in `matchers` (ASCII case-insensitive
/// for literal strings).
#[inline]
pub fn options_matches_ignore_case(matchers: &[OptionMatcher], input: &str) -> bool {
    matchers.iter().any(|m| m.is_match_ignore_ascii_case(input))
}

/// Checks whether a key-value pair `(input_name, input_value)` matches an object map under
/// `secondary[property_name]`.
pub fn options_matches_entry(
    secondary: Option<&Map<String, Value>>,
    property_name: &str,
    input_name: &str,
    input_value: &str,
) -> bool {
    let Some(map) = secondary
        .and_then(|m| m.get(property_name))
        .and_then(|v| v.as_object())
    else {
        return false;
    };
    for (k, v) in map {
        let key_matcher = OptionMatcher::parse(k);
        if key_matcher.is_match(input_name) {
            let val_matchers = parse_primary_option_matchers(v);
            if val_matchers.iter().any(|m| m.is_match(input_value)) {
                return true;
            }
        }
    }
    false
}

/// Returns `true` if `secondary[list_key]` contains the string `flag` (either as a
/// scalar string or as an element of an array).
pub fn secondary_has_flag(
    secondary: Option<&Map<String, Value>>,
    list_key: &str,
    flag: &str,
) -> bool {
    let Some(val) = secondary.and_then(|m| m.get(list_key)) else {
        return false;
    };
    match val {
        Value::String(s) => s == flag,
        Value::Array(arr) => arr.iter().any(|v| v.as_str() == Some(flag)),
        _ => false,
    }
}

/// Extracts all string elements from `secondary[key]` (supporting scalar string or array).
pub fn secondary_str_list(secondary: Option<&Map<String, Value>>, key: &str) -> Vec<String> {
    let Some(val) = secondary.and_then(|m| m.get(key)) else {
        return Vec::new();
    };
    match val {
        Value::String(s) => vec![s.clone()],
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// Extracts a boolean option from `secondary[key]`.
#[inline]
pub fn secondary_bool(secondary: Option<&Map<String, Value>>, key: &str) -> Option<bool> {
    secondary.and_then(|m| m.get(key)).and_then(|v| v.as_bool())
}

/// Extracts a string slice option from `secondary[key]`.
#[inline]
pub fn secondary_str<'a>(secondary: Option<&'a Map<String, Value>>, key: &str) -> Option<&'a str> {
    secondary.and_then(|m| m.get(key)).and_then(|v| v.as_str())
}

/// Checks whether `input` contains a literal string or array of strings from `comparison`.
///
/// Returns the matched substring from `comparison` if found, or `None` otherwise.
/// Regular expression strings (`/pattern/`) are ignored.
pub fn contains_string<'a>(input: &str, comparison: &'a Value) -> Option<&'a str> {
    match comparison {
        Value::String(s) => {
            if !s.is_empty()
                && !(s.starts_with('/') && s.ends_with('/'))
                && input.contains(s.as_str())
            {
                Some(s.as_str())
            } else {
                None
            }
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(matched) = contains_string(input, item) {
                    return Some(matched);
                }
            }
            None
        }
        _ => None,
    }
}

/// Parses a JSON map of `{ "key": ["val1", "val2"] }` from `primary` or `secondary`
/// into a vector of `(lowercase_key, lowercase_values)` pairs.
pub fn parse_string_set_map(
    primary: &Value,
    secondary: Option<&Map<String, Value>>,
) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let obj_opt = primary.as_object().or(secondary);
    let Some(obj) = obj_opt else {
        return out;
    };
    for (k, v) in obj {
        let mut set = Vec::new();
        match v {
            Value::String(s) => set.push(s.to_ascii_lowercase()),
            Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        set.push(s.to_ascii_lowercase());
                    }
                }
            }
            _ => {}
        }
        if !set.is_empty() {
            out.push((k.to_ascii_lowercase(), set));
        }
    }
    out
}

fn parse_slash_regex(raw: &str) -> Option<Regex> {
    if raw.starts_with('/') && raw.len() >= 2 {
        if let Some(last_slash) = raw.rfind('/') {
            if last_slash > 0 {
                let pattern = &raw[1..last_slash];
                let flags = &raw[last_slash + 1..];
                if flags != "" && flags != "i" {
                    return None;
                }
                let final_pat = if flags == "i" {
                    format!("(?i){pattern}")
                } else {
                    pattern.to_string()
                };
                return Regex::new(&final_pat).ok();
            }
        }
    }
    None
}

fn compile_js_regex_or_plain(raw: &str) -> Option<Regex> {
    if let Some(re) = parse_slash_regex(raw) {
        return Some(re);
    }
    Regex::new(raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().cloned().expect("object")
    }

    fn opts_match(options: Value, key: &str, input: &str) -> bool {
        options_matches(&parse_option_matchers(options.as_object(), key), input)
    }

    fn msr(inputs: &[&str], comparison: Value) -> bool {
        let matchers = parse_primary_option_matchers(&comparison);
        inputs.iter().any(|i| options_matches(&matchers, i))
    }

    #[test]
    fn is_kebab_case_cases() {
        let re = Regex::new(DEFAULT_KEBAB_PATTERN).unwrap();
        for (input, expected) in [
            ("a", true),
            ("foo", true),
            ("foo-bar", true),
            ("foo-bar-baz", true),
            ("h1", true),
            ("a-1", true),
            ("foo-1a-2", true),
            ("", false),
            ("-foo", false),
            ("1foo", false),
            ("foo-", false),
            ("foo--bar", false),
            ("Foo", false),
            ("fooBar", false),
            ("foo_bar", false),
            ("foo bar", false),
            ("café", false),
        ] {
            assert_eq!(is_kebab_case(input), expected, "{input:?}");
            assert_eq!(re.is_match(input), expected, "regex {input:?}");
        }
    }

    #[test]
    fn pattern_matcher_cases() {
        let default = PatternMatcher::default();
        assert_eq!(default.raw_pattern, DEFAULT_KEBAB_PATTERN);
        assert!(default.is_match("foo-bar") && !default.is_match("fooBar"));

        let none = PatternMatcher::new(None::<&str>);
        assert_eq!(none.raw_pattern, DEFAULT_KEBAB_PATTERN);
        assert!(none.is_match("foo-bar") && !none.is_match("foo--bar"));

        let explicit = PatternMatcher::new(Some(DEFAULT_KEBAB_PATTERN));
        assert!(explicit.is_match("a-b") && !explicit.is_match("A-b"));

        let anchored = PatternMatcher::new(Some("^foo-[a-z]+$"));
        assert_eq!(anchored.raw_pattern, "^foo-[a-z]+$");
        assert!(anchored.is_match("foo-bar"));
        assert!(!anchored.is_match("foo-Bar") && !anchored.is_match("xfoo-bar"));

        let start_only = PatternMatcher::new(Some("^foo"));
        assert!(start_only.is_match("foobar") && !start_only.is_match("xfoo"));

        let slash = PatternMatcher::new(Some("/^FOO/i"));
        assert_eq!(slash.raw_pattern, "/^FOO/i");
        assert!(slash.is_match("foo-bar") && slash.is_match("FOOBAR"));
        assert!(!slash.is_match("barfoo"));

        let slash_cs = PatternMatcher::new(Some("/bar/"));
        assert!(slash_cs.is_match("foobarbaz") && !slash_cs.is_match("BAR"));

        let invalid = PatternMatcher::new(Some("(unclosed"));
        assert_eq!(invalid.raw_pattern, "(unclosed");
        assert!(invalid.is_match("foo-bar") && !invalid.is_match("(unclosed"));

        let owned = PatternMatcher::new(Some(String::from("[0-9]+")));
        assert!(owned.is_match("123"));
    }

    #[test]
    fn pattern_matcher_unanchored() {
        assert!(PatternMatcher::new(Some("foo")).is_match("afoob"));
        assert!(PatternMatcher::new(Some("[a-z]+")).is_match("Abc"));
    }

    #[test]
    fn option_matcher_strings_upstream() {
        assert!(msr(&["bar"], json!("bar")));
        assert!(!msr(&["bar"], json!("/bar something")));
        assert!(msr(&["/bar something"], json!("/bar something")));
        assert!(msr(&["bar something/"], json!("bar something/")));
        assert!(!msr(&["bar something/"], json!("bar something//")));
        assert!(msr(&["foo", "bar"], json!("bar")));
        assert!(!msr(&["foo", "baz"], json!("bar")));
        assert!(msr(&["bar"], json!(["foo", "bar"])));
        assert!(!msr(&["bar"], json!(["foo", "baz"])));
        assert!(msr(&["foo", "baz"], json!(["foo", "bar"])));
        assert!(!msr(&["bar", "hooha"], json!(["foo", "baz"])));
    }

    #[test]
    fn option_matcher_regex_strings_upstream() {
        assert!(msr(&[".foo"], json!("/\\.foo$/")));
        assert!(msr(&["bar .foo"], json!("/\\.foo$/")));
        assert!(!msr(&["bar .foo bar"], json!("/\\.foo$/")));
        assert!(!msr(&["foo"], json!("/\\.foo$/")));
        assert!(msr(&[".foo", "bar"], json!("/\\.foo$/")));
        assert!(!msr(&["foo", "baz"], json!("/\\.foo$/")));
        assert!(msr(&[".foo"], json!(["/\\.foo$/", "/^bar/"])));
        assert!(msr(&["bar"], json!(["/\\.foo$/", "/^bar/"])));
        assert!(!msr(&["ebarz"], json!(["/\\.foo$/", "/^bar/"])));
        assert!(msr(&[".foo", "ebarz"], json!(["/\\.foo$/", "/^bar/"])));
        assert!(msr(&["bar", "foo"], json!(["/\\.foo$/", "/^bar/"])));
        assert!(!msr(&["ebarz", "foo"], json!(["/\\.foo$/", "/^bar/"])));
        assert!(!msr(&["foobar"], json!(["/FOO/"])));
        assert!(msr(&["FOOBAR"], json!(["/FOO/"])));
        assert!(!msr(&[".foo"], json!([])));
    }

    #[test]
    fn option_matcher_actual_regex_upstream() {
        let re = |p: &str| OptionMatcher::Regex(Regex::new(p).unwrap());
        let foo_end = [re(".foo$")];
        assert!(options_matches(&foo_end, ".foo"));
        assert!(options_matches(&foo_end, "bar .foo"));
        assert!(!options_matches(&foo_end, "bar .foo bar"));
        assert!(!options_matches(&foo_end, "foo"));
        let both = [re(".foo$"), re("^bar")];
        assert!([".foo", "ebarz"].iter().any(|i| options_matches(&both, i)));
        assert!(!options_matches(&[re("FOO")], "foobar"));
        assert!(options_matches(&[re("FOO")], "FOOBAR"));
    }

    #[test]
    fn option_matcher_parse_and_case() {
        assert!(matches!(OptionMatcher::parse("foo"), OptionMatcher::Exact(s) if s == "foo"));
        assert!(matches!(OptionMatcher::parse("/"), OptionMatcher::Exact(_)));
        assert!(matches!(
            OptionMatcher::parse("/foo"),
            OptionMatcher::Exact(_)
        ));
        assert!(matches!(OptionMatcher::parse(""), OptionMatcher::Exact(_)));
        assert!(matches!(
            OptionMatcher::parse("//"),
            OptionMatcher::Regex(_)
        ));
        assert!(matches!(
            OptionMatcher::parse("/foo/"),
            OptionMatcher::Regex(_)
        ));
        assert!(matches!(
            OptionMatcher::parse("/foo/i"),
            OptionMatcher::Regex(_)
        ));
        assert!(matches!(OptionMatcher::parse("/(/"), OptionMatcher::Exact(s) if s == "/(/"));

        let exact = OptionMatcher::parse("Foo");
        assert!(exact.is_match("Foo") && !exact.is_match("foo"));
        assert!(exact.is_match_ignore_ascii_case("foo") && exact.is_match_ignore_ascii_case("FOO"));
        assert!(!exact.is_match_ignore_ascii_case("fooo"));

        let regex = OptionMatcher::parse("/^Foo/");
        assert!(regex.is_match_ignore_ascii_case("Foobar"));
        assert!(!regex.is_match_ignore_ascii_case("foobar"));

        let matchers = parse_primary_option_matchers(&json!(["Foo", "/^bar/"]));
        assert!(options_matches_ignore_case(&matchers, "FOO"));
        assert!(options_matches_ignore_case(&matchers, "barx"));
        assert!(!options_matches_ignore_case(&matchers, "BARX"));
        assert!(!options_matches(&matchers, "FOO"));
        assert!(!options_matches_ignore_case(&[], "foo"));
    }

    #[test]
    fn option_matcher_only_accepts_supported_regex_flags() {
        assert!(OptionMatcher::parse("/a/b").is_match("/a/b"));
        assert!(!OptionMatcher::parse("/a/b").is_match("a"));
        assert!(OptionMatcher::parse("/a/g").is_match("/a/g"));
    }

    #[test]
    fn parse_primary_option_matchers_cases() {
        assert_eq!(parse_primary_option_matchers(&json!("a")).len(), 1);
        assert_eq!(
            parse_primary_option_matchers(&json!(["a", 1, null, "/b/"])).len(),
            2
        );
        assert!(parse_primary_option_matchers(&json!([])).is_empty());
        assert!(parse_primary_option_matchers(&json!(true)).is_empty());
        assert!(parse_primary_option_matchers(&json!(1)).is_empty());
        assert!(parse_primary_option_matchers(&json!({"a": "b"})).is_empty());
        assert!(parse_primary_option_matchers(&Value::Null).is_empty());
    }

    #[test]
    fn parse_option_matchers_cases() {
        let sec = obj(json!({"ignore": ["a", "/b/"], "one": "x", "num": 1}));
        assert_eq!(parse_option_matchers(Some(&sec), "ignore").len(), 2);
        assert_eq!(parse_option_matchers(Some(&sec), "one").len(), 1);
        assert!(parse_option_matchers(Some(&sec), "num").is_empty());
        assert!(parse_option_matchers(Some(&sec), "missing").is_empty());
        assert!(parse_option_matchers(None, "ignore").is_empty());
    }

    #[test]
    fn options_matches_upstream() {
        assert!(opts_match(json!({"foo": "bar"}), "foo", "bar"));
        assert!(!opts_match(json!({"foo": "bar"}), "foo", "BAR"));
        assert!(!opts_match(json!("not an object"), "foo", "bar"));
        assert!(!opts_match(json!({"baz": "bar"}), "foo", "bar"));
        assert!(!opts_match(json!({"foo": "baz"}), "foo", "bar"));
        assert!(opts_match(json!({"foo": ["baz", "bar"]}), "foo", "bar"));
        assert!(!opts_match(json!({"foo": ["baz", "qux"]}), "foo", "bar"));

        assert!(opts_match(json!({"foo": "/\\.bar/"}), "foo", ".bar"));
        assert!(!opts_match(json!({"foo": "/\\.baz$/"}), "foo", ".bar"));
        assert!(!opts_match(json!({"foo": "/[a-z]+/"}), "foo", "BAR"));
        assert!(opts_match(json!({"foo": "/[A-Z]+/"}), "foo", "BAR"));
        assert!(opts_match(json!({"foo": "/[a-z]+/i"}), "foo", "BAR"));
        assert!(opts_match(json!({"foo": "/[A-Z]+/i"}), "foo", "bar"));
        assert!(opts_match(
            json!({"foo": ["/\\.bar$/", ".baz"]}),
            "foo",
            ".bar"
        ));
        assert!(opts_match(
            json!({"foo": ["/\\.bar$/", ".baz"]}),
            "foo",
            ".baz"
        ));
        assert!(!opts_match(
            json!({"foo": ["/\\.bar$/", "qux"]}),
            "foo",
            ".baz"
        ));

        assert!(!opts_match(json!({}), "foo", "bar"));
        assert!(!opts_match(json!({"foo": []}), "foo", "bar"));
    }

    #[test]
    fn options_matches_entry_upstream() {
        let ome =
            |o: Value, p: &str, n: &str, v: &str| options_matches_entry(o.as_object(), p, n, v);
        assert!(ome(json!({"foo": {"bar": "baz"}}), "foo", "bar", "baz"));
        assert!(!ome(json!({"foo": {"bar": "baz"}}), "foo", "bar", "qux"));
        assert!(ome(
            json!({"foo": {"bar": ["baz", "qux"]}}),
            "foo",
            "bar",
            "qux"
        ));
        assert!(!ome(
            json!({"foo": {"bar": ["baz", "qux"]}}),
            "foo",
            "bar",
            "nope"
        ));
        assert!(ome(json!({"foo": {"/^ba/": "baz"}}), "foo", "bar", "baz"));
        assert!(ome(json!({"foo": {"bar": "/^ba/"}}), "foo", "bar", "baz"));
        assert!(!ome(
            json!({"foo": {"/^ba/": "/^qux/"}}),
            "foo",
            "bar",
            "baz"
        ));
        let options = json!({"foo": {"/^ba/": "baz", "bar": "qux"}});
        assert!(ome(options.clone(), "foo", "bar", "baz"));
        assert!(ome(options, "foo", "bar", "qux"));
        assert!(!ome(json!({}), "foo", "bar", "baz"));
        assert!(!options_matches_entry(None, "foo", "bar", "baz"));
    }

    #[test]
    fn options_matches_entry_extra() {
        let sec = obj(json!({"foo": {"bar": "baz"}, "str": "bar", "arr": ["bar"]}));
        assert!(!options_matches_entry(Some(&sec), "str", "bar", "baz"));
        assert!(!options_matches_entry(Some(&sec), "arr", "bar", "baz"));
        assert!(!options_matches_entry(Some(&sec), "foo", "BAR", "baz"));
        assert!(!options_matches_entry(Some(&sec), "foo", "bar", "BAZ"));
        let sec = obj(json!({"foo": {"/^BA/i": "/Z$/i", "x": 1}}));
        assert!(options_matches_entry(Some(&sec), "foo", "bar", "baz"));
        assert!(!options_matches_entry(Some(&sec), "foo", "x", "1"));
    }

    #[test]
    fn secondary_has_flag_cases() {
        let sec =
            obj(json!({"ignore": ["comments", "after-comment"], "one": "first-nested", "num": 1}));
        assert!(secondary_has_flag(Some(&sec), "ignore", "comments"));
        assert!(secondary_has_flag(Some(&sec), "ignore", "after-comment"));
        assert!(!secondary_has_flag(Some(&sec), "ignore", "Comments"));
        assert!(!secondary_has_flag(Some(&sec), "ignore", "comment"));
        assert!(secondary_has_flag(Some(&sec), "one", "first-nested"));
        assert!(!secondary_has_flag(Some(&sec), "num", "1"));
        assert!(!secondary_has_flag(Some(&sec), "missing", "comments"));
        assert!(!secondary_has_flag(None, "ignore", "comments"));
    }

    #[test]
    fn secondary_str_list_cases() {
        let sec = obj(json!({"a": ["x", 1, "y", null], "b": "z", "c": true, "d": []}));
        assert_eq!(secondary_str_list(Some(&sec), "a"), ["x", "y"]);
        assert_eq!(secondary_str_list(Some(&sec), "b"), ["z"]);
        assert!(secondary_str_list(Some(&sec), "c").is_empty());
        assert!(secondary_str_list(Some(&sec), "d").is_empty());
        assert!(secondary_str_list(Some(&sec), "missing").is_empty());
        assert!(secondary_str_list(None, "a").is_empty());
    }

    #[test]
    fn secondary_bool_cases() {
        let sec = obj(json!({"t": true, "f": false, "s": "true", "n": 1}));
        assert_eq!(secondary_bool(Some(&sec), "t"), Some(true));
        assert_eq!(secondary_bool(Some(&sec), "f"), Some(false));
        assert_eq!(secondary_bool(Some(&sec), "s"), None);
        assert_eq!(secondary_bool(Some(&sec), "n"), None);
        assert_eq!(secondary_bool(Some(&sec), "missing"), None);
        assert_eq!(secondary_bool(None, "t"), None);
    }

    #[test]
    fn secondary_str_cases() {
        let sec = obj(json!({"s": "always", "e": "", "a": ["x"], "b": false}));
        assert_eq!(secondary_str(Some(&sec), "s"), Some("always"));
        assert_eq!(secondary_str(Some(&sec), "e"), Some(""));
        assert_eq!(secondary_str(Some(&sec), "a"), None);
        assert_eq!(secondary_str(Some(&sec), "b"), None);
        assert_eq!(secondary_str(Some(&sec), "missing"), None);
        assert_eq!(secondary_str(None, "s"), None);
    }

    #[test]
    fn contains_string_upstream() {
        assert_eq!(contains_string("bar", &json!("bar")), Some("bar"));
        assert_eq!(
            contains_string("foo bar something", &json!("bar")),
            Some("bar")
        );
        assert_eq!(contains_string("bar", &json!("foo")), None);
        assert_eq!(
            contains_string("/bar something", &json!("/bar")),
            Some("/bar")
        );
        assert_eq!(
            contains_string("bar something/", &json!("something/")),
            Some("something/")
        );
        assert_eq!(contains_string("/bar/", &json!("/bar/")), None);
        assert_eq!(contains_string("/bar/ something", &json!("/bar/")), None);
        assert_eq!(contains_string("bar", &json!("")), None);
        assert_eq!(contains_string("bar", &Value::Null), None);

        assert_eq!(contains_string("bar", &json!(["foo", "bar"])), Some("bar"));
        assert_eq!(
            contains_string("foo baz something", &json!(["bar", "baz"])),
            Some("baz")
        );
        assert_eq!(
            contains_string("foo bar", &json!(["bar", "foo"])),
            Some("bar")
        );
        assert_eq!(contains_string("bar", &json!(["foo", "baz"])), None);
        assert_eq!(contains_string("/bar/", &json!(["/bar/"])), None);
        assert_eq!(
            contains_string("/bar/ something", &json!(["/bar/", "foo"])),
            None
        );
        assert_eq!(contains_string("bar", &json!([])), None);
    }

    #[test]
    fn contains_string_extra() {
        assert_eq!(contains_string("bar", &json!(1)), None);
        assert_eq!(
            contains_string("bar", &json!([1, null, "", "ar"])),
            Some("ar")
        );
        assert_eq!(contains_string("bar", &json!([["b"]])), Some("b"));
        assert_eq!(contains_string("BAR", &json!("bar")), None);
        assert_eq!(contains_string("", &json!("a")), None);
        assert_eq!(contains_string("café", &json!("é")), Some("é"));
    }

    #[test]
    fn contains_string_regex_flags_are_literal() {
        assert_eq!(
            contains_string("x /bar/i y", &json!("/bar/i")),
            Some("/bar/i")
        );
    }

    #[test]
    fn parse_string_set_map_cases() {
        let sorted = |mut v: Vec<(String, Vec<String>)>| {
            v.sort();
            v
        };
        let primary =
            json!({"Font-Face": ["Font-Family", 1, "SRC"], "page": "Margin", "empty": [], "n": 2});
        assert_eq!(
            sorted(parse_string_set_map(&primary, None)),
            vec![
                (
                    "font-face".to_string(),
                    vec!["font-family".to_string(), "src".to_string()]
                ),
                ("page".to_string(), vec!["margin".to_string()]),
            ]
        );
        let sec = obj(json!({"Property": ["Syntax"]}));
        assert_eq!(
            parse_string_set_map(&json!(true), Some(&sec)),
            vec![("property".to_string(), vec!["syntax".to_string()])]
        );
        assert_eq!(
            parse_string_set_map(&json!({"a": "b"}), Some(&sec)).len(),
            1
        );
        assert_eq!(
            parse_string_set_map(&json!({"a": "b"}), Some(&sec))[0].0,
            "a"
        );
        assert!(parse_string_set_map(&json!({}), Some(&sec)).is_empty());
        assert!(parse_string_set_map(&json!(null), None).is_empty());
        assert!(parse_string_set_map(&json!(["a"]), None).is_empty());
    }
}

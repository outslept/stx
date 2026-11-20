use regex::Regex;
use serde_json::{Map, Value};

pub use crate::rules::value_utils::{DEFAULT_KEBAB_PATTERN, PatternMatcher, is_kebab_case};

/// Matcher used by Stylelint's `optionsMatches(options, propertyName, input)`:
/// - `/pattern/flags` is compiled as a Regex
/// - Plain string `"foo"` matches exact equality (`input == "foo"`)
#[derive(Debug, Clone)]
pub enum OptionMatcher {
    Exact(String),
    Regex(Regex),
}

impl OptionMatcher {
    pub fn parse(raw: &str) -> Self {
        if raw.starts_with('/') {
            if let Some(last_slash) = raw.rfind('/') {
                if last_slash > 0 {
                    let body = &raw[1..last_slash];
                    let flags = &raw[last_slash + 1..];
                    let pattern = if flags.contains('i') {
                        format!("(?i){body}")
                    } else {
                        body.to_string()
                    };
                    if let Ok(re) = Regex::new(&pattern) {
                        return Self::Regex(re);
                    }
                }
            }
        }
        Self::Exact(raw.to_string())
    }

    pub fn is_match(&self, input: &str) -> bool {
        match self {
            Self::Exact(s) => s == input,
            Self::Regex(re) => re.is_match(input),
        }
    }

    pub fn is_match_ignore_ascii_case(&self, input: &str) -> bool {
        match self {
            Self::Exact(s) => s.eq_ignore_ascii_case(input),
            Self::Regex(re) => re.is_match(input),
        }
    }
}

/// Parses an array of strings/regex-strings from `secondary[key]` into `Vec<OptionMatcher>`.
pub fn parse_option_matchers(
    secondary: Option<&Map<String, Value>>,
    key: &str,
) -> Vec<OptionMatcher> {
    let Some(sec) = secondary else {
        return Vec::new();
    };
    let Some(arr) = sec.get(key).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str())
        .map(OptionMatcher::parse)
        .collect()
}

/// Equivalent to Stylelint's `optionsMatches(secondaryOptions, key, input)`.
pub fn options_matches(matchers: &[OptionMatcher], input: &str) -> bool {
    matchers.iter().any(|m| m.is_match(input))
}

/// Case-insensitive variant of `options_matches`.
pub fn options_matches_ignore_case(matchers: &[OptionMatcher], input: &str) -> bool {
    matchers.iter().any(|m| m.is_match_ignore_ascii_case(input))
}

/// Checks if `secondary[key]` array contains the given string flag.
pub fn secondary_has_flag(secondary: Option<&Map<String, Value>>, key: &str, flag: &str) -> bool {
    secondary
        .and_then(|sec| sec.get(key))
        .and_then(|v| v.as_array())
        .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some(flag)))
}

/// Extracts a list of strings from `secondary[key]`.
pub fn secondary_str_list(secondary: Option<&Map<String, Value>>, key: &str) -> Vec<String> {
    secondary
        .and_then(|sec| sec.get(key))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Extracts a boolean option from `secondary[key]`, returning `default` if absent.
pub fn secondary_bool(secondary: Option<&Map<String, Value>>, key: &str, default: bool) -> bool {
    secondary
        .and_then(|sec| sec.get(key))
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// Extracts a string option from `secondary[key]`.
pub fn secondary_str<'a>(secondary: Option<&'a Map<String, Value>>, key: &str) -> Option<&'a str> {
    secondary
        .and_then(|sec| sec.get(key))
        .and_then(|v| v.as_str())
}

/// Parses a primary option that accepts `string | RegExp | Array<string | RegExp>`
/// (`rule.primaryOptionArray = true` in Stylelint) into `Vec<OptionMatcher>`.
pub fn parse_primary_option_matchers(primary: &Value) -> Vec<OptionMatcher> {
    match primary {
        Value::String(s) => vec![OptionMatcher::parse(s)],
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(OptionMatcher::parse)
            .collect(),
        _ => Vec::new(),
    }
}

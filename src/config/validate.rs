use super::{Config, RuleSetting};
use crate::diagnostic::Severity;
use serde_json::Value;

impl Config {
    pub(super) fn validate_rule_name(
        name: &str,
        setting: Option<&RuleSetting>,
        errors: &mut Vec<String>,
    ) {
        if !crate::rules::is_known_rule(name, setting) {
            errors.push(format!("Unknown rule {name}"));
        }
    }

    pub(super) fn validate_rule_setting(
        name: &str,
        setting: &RuleSetting,
        errors: &mut Vec<String>,
    ) {
        let expected = match name {
            "number-max-precision" | "max-nesting-depth" | "selector-max-type" => {
                setting.primary.as_u64().is_some()
            }
            "property-no-unknown" | "length-zero-no-unit" | "unit-no-unknown" => {
                setting.primary == Value::Bool(true)
            }
            _ => true,
        };
        if !expected {
            errors.push(format!("Invalid primary option for rule {name}"));
        }
        let allowed = match name {
            "property-no-unknown" => Some(
                &[
                    "ignoreProperties",
                    "checkPrefixed",
                    "ignoreSelectors",
                    "ignoreAtRules",
                ][..],
            ),
            "selector-max-type" => Some(&["ignore", "ignoreTypes"][..]),
            "length-zero-no-unit" => {
                Some(&["ignore", "ignoreFunctions", "ignorePreludeOfAtRules"][..])
            }
            "number-max-precision" => {
                Some(&["ignoreProperties", "ignoreUnits", "insideFunctions"][..])
            }
            "max-nesting-depth" => Some(
                &[
                    "ignore",
                    "ignoreAtRules",
                    "ignoreRules",
                    "ignorePseudoClasses",
                ][..],
            ),
            "unit-no-unknown" => Some(&["ignoreUnits", "ignoreFunctions"][..]),
            _ => None,
        };
        if let Some(secondary) = &setting.secondary {
            if let Some(value) = secondary.get("severity")
                && !value
                    .as_str()
                    .is_some_and(|severity| matches!(severity, "warning" | "error"))
            {
                errors.push(format!("Invalid severity for rule {name}"));
            }
            if secondary
                .get("message")
                .is_some_and(|message| !message.is_string())
            {
                errors.push(format!("Invalid message option for rule {name}"));
            }
            for (key, value) in secondary {
                if matches!(
                    key.as_str(),
                    "severity" | "message" | "url" | "reportDisables" | "disableFix"
                ) {
                    continue;
                }
                if allowed.is_some_and(|keys| !keys.contains(&key.as_str()))
                    || (allowed.is_none() && !Self::is_secondary_option_name(key))
                {
                    errors.push(format!("Invalid secondary option {key:?} for rule {name}"));
                } else if allowed.is_some() && !Self::valid_secondary_value(name, key, value) {
                    errors.push(format!(
                        "Invalid value for secondary option {key:?} of rule {name}"
                    ));
                }
            }
        }
    }

    fn is_secondary_option_name(key: &str) -> bool {
        matches!(
            key,
            "ignore"
                | "except"
                | "ignoreAtRules"
                | "ignoreRules"
                | "ignoreFunctions"
                | "ignoreProperties"
                | "ignoreUnits"
                | "ignoreValues"
                | "ignoreKeywords"
                | "ignorePseudoClasses"
                | "ignoreMediaFeatureNames"
                | "ignoreMediaFeatureNameValues"
                | "ignoreSelectors"
                | "ignoreTypes"
                | "ignoreAttributes"
                | "ignoreNamespaces"
                | "ignorePseudoElements"
                | "ignoreShorthands"
                | "ignoreLonghands"
                | "ignoreAnnotations"
                | "ignoreMediaTypes"
                | "exceptProperties"
                | "splitList"
                | "disallowInList"
                | "ignoreFontFamilyNames"
                | "ignoreFontFamilies"
                | "ignoreComments"
                | "ignorePreludeOfAtRules"
                | "ignoreAfterCombinators"
                | "checkPrefixed"
                | "camelCaseSvgKeywords"
                | "insideFunctions"
                | "severity"
                | "message"
                | "url"
                | "reportDisables"
                | "disableFix"
        )
    }

    fn valid_secondary_value(name: &str, key: &str, value: &Value) -> bool {
        let string_or_list = || match value {
            Value::String(_) => true,
            Value::Array(values) => values.iter().all(Value::is_string),
            _ => false,
        };
        match (name, key) {
            ("property-no-unknown", "checkPrefixed") => value.is_boolean(),
            ("number-max-precision", "insideFunctions") => value
                .as_object()
                .is_some_and(|values| values.values().all(|value| value.as_u64().is_some())),
            ("selector-max-type", "ignore") => Self::string_list_has_only(
                value,
                &[
                    "descendant",
                    "child",
                    "compounded",
                    "next-sibling",
                    "custom-elements",
                ],
            ),
            ("max-nesting-depth", "ignore") => {
                Self::string_list_has_only(value, &["blockless-at-rules", "pseudo-classes"])
            }
            ("length-zero-no-unit", "ignore") => {
                Self::string_list_has_only(value, &["custom-properties"])
            }
            _ => string_or_list(),
        }
    }

    fn string_list_has_only(value: &Value, allowed: &[&str]) -> bool {
        match value {
            Value::String(value) => allowed.contains(&value.as_str()),
            Value::Array(values) => values
                .iter()
                .all(|value| value.as_str().is_some_and(|value| allowed.contains(&value))),
            _ => false,
        }
    }

    pub(crate) fn parse_rule_value(
        name: &str,
        val: Value,
        default_severity: Severity,
    ) -> Option<RuleSetting> {
        let (primary, secondary) = match val {
            Value::Null => return None,
            Value::String(ref s) if s == "off" => return None,
            Value::Array(items) if matches!(items.first(), Some(Value::Null)) => return None,
            Value::Array(items)
                if list_primary(name)
                    && !matches!(items.as_slice(), [Value::Array(_)])
                    && !matches!(items.as_slice(), [first, Value::Object(_)] if !first.is_object()) =>
            {
                (Value::Array(items), None)
            }
            Value::Array(items) => {
                let mut items = items.into_iter();
                let primary = items.next().unwrap_or(Value::Null);
                let secondary = match items.next() {
                    Some(Value::Object(map)) => Some(map),
                    _ => None,
                };
                (primary, secondary)
            }
            primary => (primary, None),
        };
        let severity = match secondary
            .as_ref()
            .and_then(|map| map.get("severity"))
            .and_then(Value::as_str)
        {
            Some("warning") => Severity::Warning,
            Some("error") => Severity::Error,
            _ => default_severity,
        };
        Some(RuleSetting {
            primary,
            secondary,
            severity,
        })
    }
}

fn list_primary(name: &str) -> bool {
    matches!(
        name,
        "at-rule-allowed-list"
            | "at-rule-disallowed-list"
            | "comment-word-disallowed-list"
            | "function-allowed-list"
            | "function-disallowed-list"
            | "function-url-scheme-allowed-list"
            | "function-url-scheme-disallowed-list"
            | "media-feature-name-allowed-list"
            | "media-feature-name-disallowed-list"
            | "property-allowed-list"
            | "property-disallowed-list"
            | "rule-nesting-at-rule-required-list"
            | "selector-attribute-name-disallowed-list"
            | "selector-attribute-operator-allowed-list"
            | "selector-attribute-operator-disallowed-list"
            | "selector-combinator-allowed-list"
            | "selector-combinator-disallowed-list"
            | "selector-disallowed-list"
            | "selector-pseudo-class-allowed-list"
            | "selector-pseudo-class-disallowed-list"
            | "selector-pseudo-element-allowed-list"
            | "selector-pseudo-element-disallowed-list"
            | "unit-allowed-list"
            | "unit-disallowed-list"
            | "value-type-custom-property-required-list"
    )
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn validates_known_rule_options() {
        let config = Config::from_json_str(
            r#"{"rules":{"color-hex-lenght":true,"number-max-precision":"many","property-no-unknown":[true,{"typo":true}]}}"#,
        )
        .unwrap();
        assert!(
            config
                .validation_errors
                .iter()
                .any(|message| message.contains("Unknown rule color-hex-lenght"))
        );
        assert!(
            config
                .validation_errors
                .iter()
                .any(|message| message.contains("number-max-precision"))
        );
        assert!(
            config
                .validation_errors
                .iter()
                .any(|message| message.contains("typo"))
        );
    }

    #[test]
    fn recognizes_rules_with_special_primary_options() {
        let config =
            Config::from_json_str(r#"{"rules":{"color-function-alias-notation":"with-alpha"}}"#)
                .unwrap();
        assert!(config.validation_errors.is_empty());
    }
}

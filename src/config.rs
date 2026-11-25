use crate::diagnostic::Severity;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct RuleSetting {
    pub primary: Value,
    pub secondary: Option<Map<String, Value>>,
    pub severity: Severity,
}

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub rules: HashMap<String, Option<RuleSetting>>,
    pub default_severity: Severity,
    pub validation_errors: Vec<String>,
    pub ignore_files: Vec<String>,
    pub overrides: Vec<ConfigOverride>,
    pub base_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ConfigOverride {
    pub files: Vec<String>,
    pub rules: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConfig {
    extends: Option<Value>,
    rules: Option<HashMap<String, Value>>,
    default_severity: Option<String>,
    ignore_files: Option<Value>,
    overrides: Option<Vec<RawOverride>>,
}

#[derive(Debug, Deserialize)]
struct RawOverride {
    files: Value,
    rules: Option<HashMap<String, Value>>,
}

const CONFIG_FILES: &[&str] = &[
    ".stylelintrc",
    ".stylelintrc.json",
    "stylelint.config.json",
    ".stxrc",
    ".stxrc.json",
    ".stylelintrc.yaml",
    ".stylelintrc.yml",
    ".stylelintrc.js",
    ".stylelintrc.cjs",
    ".stylelintrc.mjs",
    "stylelint.config.js",
    "stylelint.config.cjs",
    "stylelint.config.mjs",
    "stylelint.config.ts",
    "stylelint.config.cts",
    "stylelint.config.mts",
];

impl Config {
    pub fn discover(start_dir: &Path) -> Option<PathBuf> {
        let mut current = Some(start_dir);
        while let Some(dir) = current {
            for name in CONFIG_FILES {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            let pkg = dir.join("package.json");
            if pkg.is_file()
                && let Ok(content) = fs::read_to_string(&pkg)
                && let Ok(json) = serde_json::from_str::<Value>(&content)
                && json.get("stylelint").is_some()
            {
                return Some(pkg);
            }
            current = dir.parent();
        }
        None
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext,
                    "js" | "cjs" | "mjs" | "ts" | "cts" | "mts" | "yaml" | "yml"
                )
            })
        {
            return Err(format!(
                "Unsupported config format in {}: use JSON",
                path.display()
            ));
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config {}: {e}", path.display()))?;
        if path.file_name().and_then(|n| n.to_str()) == Some("package.json") {
            let json: Value = serde_json::from_str(&content)
                .map_err(|e| format!("Invalid JSON in {}: {e}", path.display()))?;
            if let Some(stylelint_cfg) = json.get("stylelint") {
                let mut config = Self::from_json_str(&stylelint_cfg.to_string())?;
                config.base_dir = path.parent().map(Path::to_path_buf);
                return Ok(config);
            }
            return Ok(Self {
                base_dir: path.parent().map(Path::to_path_buf),
                ..Self::default()
            });
        }
        let mut config = Self::from_json_str(&content)?;
        config.base_dir = path.parent().map(Path::to_path_buf);
        Ok(config)
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, String> {
        serde_json::from_str(json_str)
            .map(Self::from_raw)
            .map_err(|e| format!("Invalid config JSON: {e}"))
    }

    pub fn from_json_object(object: Map<String, Value>) -> Result<Self, String> {
        serde_json::from_value(Value::Object(object))
            .map(Self::from_raw)
            .map_err(|e| format!("Invalid config JSON: {e}"))
    }

    fn from_raw(raw: RawConfig) -> Self {
        let mut config = Self {
            default_severity: match raw.default_severity.as_deref() {
                Some("warning") => Severity::Warning,
                _ => Severity::Error,
            },
            ..Self::default()
        };
        if let Some(severity) = raw.default_severity.as_deref()
            && !matches!(severity, "warning" | "error")
        {
            config
                .validation_errors
                .push(format!("Invalid defaultSeverity value {severity:?}"));
        }
        if let Some(extends) = raw.extends {
            match extends {
                Value::String(s) => config.apply_extend(&s),
                Value::Array(arr) => {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            config.apply_extend(s);
                        }
                    }
                }
                _ => {}
            }
        }
        for setting in config.rules.values_mut().flatten() {
            setting.severity = config.default_severity;
        }
        if let Some(rules) = raw.rules {
            for (name, val) in rules {
                let parsed = Self::parse_rule_value(&name, val, config.default_severity);
                Self::validate_rule_name(&name, parsed.as_ref(), &mut config.validation_errors);
                if let Some(setting) = parsed.as_ref() {
                    Self::validate_rule_setting(&name, setting, &mut config.validation_errors);
                }
                config.rules.insert(name, parsed);
            }
        }
        config.ignore_files = match raw.ignore_files {
            Some(Value::String(pattern)) => vec![pattern],
            Some(Value::Array(patterns)) => patterns
                .into_iter()
                .filter_map(|pattern| pattern.as_str().map(str::to_owned))
                .collect(),
            Some(_) => {
                config
                    .validation_errors
                    .push("ignoreFiles must be a string or array of strings".into());
                Vec::new()
            }
            None => Vec::new(),
        };
        for override_config in raw.overrides.unwrap_or_default() {
            let files = match override_config.files {
                Value::String(file) => vec![file],
                Value::Array(files) => files
                    .into_iter()
                    .filter_map(|file| file.as_str().map(str::to_owned))
                    .collect(),
                _ => Vec::new(),
            };
            let mut rules = HashMap::new();
            for (name, value) in override_config.rules.unwrap_or_default() {
                let setting = Self::parse_rule_value(&name, value.clone(), config.default_severity);
                Self::validate_rule_name(&name, setting.as_ref(), &mut config.validation_errors);
                if let Some(setting) = setting {
                    Self::validate_rule_setting(&name, &setting, &mut config.validation_errors);
                }
                rules.insert(name, value);
            }
            config.overrides.push(ConfigOverride { files, rules });
        }
        config
    }

    pub fn recommended_preset() -> Self {
        let mut cfg = Self::default();
        let recommended = [
            "annotation-no-unknown",
            "at-rule-descriptor-no-unknown",
            "at-rule-descriptor-value-no-unknown",
            "at-rule-no-deprecated",
            "at-rule-no-unknown",
            "at-rule-prelude-no-invalid",
            "block-no-empty",
            "block-no-redundant-nested-style-rules",
            "color-no-invalid-hex",
            "comment-no-empty",
            "custom-property-no-missing-var-function",
            "declaration-block-no-duplicate-custom-properties",
            "declaration-block-no-duplicate-properties",
            "declaration-block-no-shorthand-property-overrides",
            "font-family-no-duplicate-names",
            "font-family-no-missing-generic-family-keyword",
            "function-calc-no-unspaced-operator",
            "function-linear-gradient-no-nonstandard-direction",
            "function-no-unknown",
            "keyframe-block-no-duplicate-selectors",
            "keyframe-declaration-no-important",
            "media-feature-name-no-unknown",
            "media-query-no-invalid",
            "named-grid-areas-no-invalid",
            "no-descending-specificity",
            "no-duplicate-at-import-rules",
            "no-duplicate-selectors",
            "no-empty-source",
            "no-invalid-double-slash-comments",
            "no-invalid-position-at-import-rule",
            "no-invalid-position-declaration",
            "no-irregular-whitespace",
            "property-no-unknown",
            "selector-anb-no-unmatchable",
            "selector-pseudo-class-no-unknown",
            "selector-pseudo-element-no-unknown",
            "selector-type-no-unknown",
            "string-no-newline",
            "unit-no-unknown",
        ];
        for name in recommended {
            cfg.enable(name);
        }
        cfg
    }

    pub fn standard_preset() -> Self {
        let mut cfg = Self::recommended_preset();
        let standard_rules = [
            ("alpha-value-notation", Value::String("percentage".into())),
            ("at-rule-empty-line-before", Value::String("always".into())),
            ("at-rule-no-vendor-prefix", Value::Bool(true)),
            (
                "color-function-alias-notation",
                Value::String("without-alpha".into()),
            ),
            ("color-function-notation", Value::String("modern".into())),
            ("color-hex-length", Value::String("short".into())),
            ("comment-empty-line-before", Value::String("always".into())),
            ("comment-whitespace-inside", Value::String("always".into())),
            (
                "custom-property-empty-line-before",
                Value::String("always".into()),
            ),
            (
                "custom-media-pattern",
                Value::String("^([a-z][a-z0-9]*)(-[a-z0-9]+)*$".into()),
            ),
            (
                "custom-property-pattern",
                Value::String("^([a-z][a-z0-9]*)(-[a-z0-9]+)*$".into()),
            ),
            (
                "container-name-pattern",
                Value::String("^(--)?([a-z][a-z0-9]*)(-[a-z0-9]+)*$".into()),
            ),
            (
                "declaration-block-no-redundant-longhand-properties",
                Value::Bool(true),
            ),
            (
                "declaration-block-single-line-max-declarations",
                Value::Number(1.into()),
            ),
            (
                "declaration-empty-line-before",
                Value::String("always".into()),
            ),
            (
                "font-family-name-quotes",
                Value::String("always-where-recommended".into()),
            ),
            ("function-name-case", Value::String("lower".into())),
            ("function-url-quotes", Value::String("always".into())),
            ("hue-degree-notation", Value::String("angle".into())),
            ("import-notation", Value::String("url".into())),
            (
                "keyframe-selector-notation",
                Value::String("percentage-unless-within-keyword-only-block".into()),
            ),
            (
                "keyframes-name-pattern",
                Value::String("^([a-z][a-z0-9]*)(-[a-z0-9]+)*$".into()),
            ),
            (
                "layer-name-pattern",
                Value::String(
                    "^([a-z][a-z0-9]*)(-[a-z0-9]+)*(\\.([a-z][a-z0-9]*)(-[a-z0-9]+)*)*$".into(),
                ),
            ),
            ("length-zero-no-unit", Value::Bool(true)),
            ("lightness-notation", Value::String("percentage".into())),
            ("media-feature-name-no-vendor-prefix", Value::Bool(true)),
            (
                "media-feature-range-notation",
                Value::String("context".into()),
            ),
            ("number-max-precision", Value::Number(4.into())),
            ("property-no-vendor-prefix", Value::Bool(true)),
            (
                "rule-empty-line-before",
                Value::String("always-multi-line".into()),
            ),
            ("selector-attribute-quotes", Value::String("always".into())),
            (
                "selector-class-pattern",
                Value::String("^([a-z][a-z0-9]*)(-[a-z0-9]+)*$".into()),
            ),
            (
                "selector-id-pattern",
                Value::String("^([a-z][a-z0-9]*)(-[a-z0-9]+)*$".into()),
            ),
            ("selector-no-vendor-prefix", Value::Bool(true)),
            ("selector-not-notation", Value::String("complex".into())),
            (
                "selector-pseudo-element-colon-notation",
                Value::String("double".into()),
            ),
            ("selector-type-case", Value::String("lower".into())),
            ("shorthand-property-no-redundant-values", Value::Bool(true)),
            ("value-keyword-case", Value::String("lower".into())),
            ("value-no-vendor-prefix", Value::Bool(true)),
        ];
        for (name, primary) in standard_rules {
            cfg.rules.insert(
                name.to_string(),
                Some(RuleSetting {
                    primary,
                    secondary: None,
                    severity: Severity::Error,
                }),
            );
        }
        cfg
    }

    pub fn for_file(&self, path: &Path) -> Self {
        let root = self.base_dir.as_deref().unwrap_or_else(|| Path::new("."));
        let mut config = self.clone();
        for override_config in &self.overrides {
            let mut builder = ignore::overrides::OverrideBuilder::new(root);
            let mut valid = true;
            for pattern in &override_config.files {
                if builder.add(pattern).is_err() {
                    valid = false;
                    break;
                }
            }
            if !valid
                || !builder
                    .build()
                    .is_ok_and(|matcher| matcher.matched(path, false).is_whitelist())
            {
                continue;
            }
            for (name, value) in &override_config.rules {
                config.rules.insert(
                    name.clone(),
                    Self::parse_rule_value(name, value.clone(), config.default_severity),
                );
            }
        }
        config
    }

    pub fn enable(&mut self, rule_name: &str) {
        self.rules.insert(
            rule_name.to_string(),
            Some(RuleSetting {
                primary: Value::Bool(true),
                secondary: None,
                severity: Severity::Error,
            }),
        );
    }

    fn apply_extend(&mut self, extend_name: &str) {
        let base = if extend_name.contains("standard") {
            Self::standard_preset()
        } else if extend_name.contains("recommended") {
            Self::recommended_preset()
        } else {
            return;
        };
        for (k, v) in base.rules {
            self.rules.insert(k, v);
        }
    }

    fn validate_rule_name(name: &str, setting: Option<&RuleSetting>, errors: &mut Vec<String>) {
        if !crate::rules::is_known_rule(name, setting) {
            errors.push(format!("Unknown rule {name}"));
        }
    }

    fn validate_rule_setting(name: &str, setting: &RuleSetting, errors: &mut Vec<String>) {
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
    use crate::rules::build_rules;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn stylelint_config_shapes() {
        let cfg = Config::from_json_str(
            r#"{"rules": {
                "at-rule-property-required-list": {"font-face": ["font-family"]},
                "function-allowed-list": ["rgb", "url"],
                "unit-allowed-list": [["px", "em"], {"severity": "warning"}],
                "unit-disallowed-list": [],
                "property-allowed-list": [null]
            }}"#,
        )
        .unwrap();
        let setting = |name: &str| cfg.rules[name].as_ref().unwrap();
        assert_eq!(
            setting("at-rule-property-required-list").primary,
            json!({"font-face": ["font-family"]})
        );
        assert_eq!(
            setting("function-allowed-list").primary,
            json!(["rgb", "url"])
        );
        let units = setting("unit-allowed-list");
        assert_eq!(units.primary, json!(["px", "em"]));
        assert_eq!(units.severity.as_str(), "warning");
        assert_eq!(setting("unit-disallowed-list").primary, json!([]));
        assert!(cfg.rules["property-allowed-list"].is_none());
    }

    #[test]
    fn default_severity_and_per_rule_severity() {
        let config = Config::from_json_str(
            r#"{"defaultSeverity":"warning","rules":{"block-no-empty":true,"color-hex-length":["short",{"severity":"error"}]}}"#,
        )
        .unwrap();
        assert_eq!(config.default_severity.as_str(), "warning");
        assert_eq!(
            config.rules["block-no-empty"]
                .as_ref()
                .unwrap()
                .severity
                .as_str(),
            "warning"
        );
        assert_eq!(
            config.rules["color-hex-length"]
                .as_ref()
                .unwrap()
                .severity
                .as_str(),
            "error"
        );
    }

    #[test]
    fn default_severity_applies_to_extended_rules() {
        let config = Config::from_json_str(
            r#"{"extends":"stylelint:recommended","defaultSeverity":"warning"}"#,
        )
        .unwrap();
        assert_eq!(
            config.rules["block-no-empty"]
                .as_ref()
                .unwrap()
                .severity
                .as_str(),
            "warning"
        );
    }

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

    #[test]
    fn file_overrides_replace_matching_rule_settings() {
        let config = Config::from_json_str(
            r#"{"rules":{"color-hex-length":"short"},"overrides":[{"files":["*.scss"],"rules":{"color-hex-length":"long"}}]}"#,
        )
        .unwrap();
        let file_config = config.for_file(Path::new("sample.scss"));
        assert_eq!(
            file_config.rules["color-hex-length"]
                .as_ref()
                .unwrap()
                .primary,
            json!("long")
        );
        assert_eq!(
            config.rules["color-hex-length"].as_ref().unwrap().primary,
            json!("short")
        );
    }

    #[test]
    fn default_preset_checks_unknown_descriptors() {
        assert!(
            build_rules(&Config::standard_preset())
                .iter()
                .any(|configured| { configured.rule.name() == "at-rule-descriptor-no-unknown" })
        );
    }
}

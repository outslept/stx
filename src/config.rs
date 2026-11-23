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

impl Default for Severity {
    fn default() -> Self {
        Self::Error
    }
}

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub rules: HashMap<String, Option<RuleSetting>>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    extends: Option<Value>,
    rules: Option<HashMap<String, Value>>,
}

const CONFIG_FILES: &[&str] = &[
    ".stylelintrc",
    ".stylelintrc.json",
    "stylelint.config.json",
    ".stxrc",
    ".stxrc.json",
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
            if pkg.is_file() {
                if let Ok(content) = fs::read_to_string(&pkg) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        if json.get("stylelint").is_some() {
                            return Some(pkg);
                        }
                    }
                }
            }
            current = dir.parent();
        }
        None
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config {}: {e}", path.display()))?;
        if path.file_name().and_then(|n| n.to_str()) == Some("package.json") {
            let json: Value = serde_json::from_str(&content)
                .map_err(|e| format!("Invalid JSON in {}: {e}", path.display()))?;
            if let Some(stylelint_cfg) = json.get("stylelint") {
                return Self::from_json_str(&stylelint_cfg.to_string());
            }
            return Ok(Self::default());
        }
        Self::from_json_str(&content)
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, String> {
        let raw: RawConfig =
            serde_json::from_str(json_str).map_err(|e| format!("Invalid config JSON: {e}"))?;
        let mut config = Self::default();
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
        if let Some(rules) = raw.rules {
            for (name, val) in rules {
                let parsed = Self::parse_rule_value(val);
                config.rules.insert(name, parsed);
            }
        }
        Ok(config)
    }

    pub fn recommended_preset() -> Self {
        let mut cfg = Self::default();
        let recommended = [
            "annotation-no-unknown",
            "at-Rule-descriptor-no-unknown",
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

    pub(crate) fn parse_rule_value(val: Value) -> Option<RuleSetting> {
        match val {
            Value::Null | Value::Bool(false) => None,
            Value::String(ref s) if s == "off" => None,
            Value::Bool(true) | Value::String(_) | Value::Number(_) => Some(RuleSetting {
                primary: val,
                secondary: None,
                severity: Severity::Error,
            }),
            Value::Array(mut arr) => {
                if arr.is_empty() {
                    return None;
                }
                let first = arr.remove(0);
                if first.is_null() || first == Value::Bool(false) {
                    return None;
                }
                let mut secondary = None;
                let mut severity = Severity::Error;
                if !arr.is_empty() {
                    if let Value::Object(map) = arr.remove(0) {
                        if let Some(sev_str) = map.get("severity").and_then(|v| v.as_str()) {
                            if sev_str == "warning" {
                                severity = Severity::Warning;
                            }
                        }
                        secondary = Some(map);
                    }
                }
                Some(RuleSetting {
                    primary: first,
                    secondary,
                    severity,
                })
            }
            _ => None,
        }
    }
}

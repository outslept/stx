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
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    extends: Option<Value>,
    #[serde(default, rename = "defaultSeverity")]
    default_severity: Option<String>,
    #[serde(default)]
    rules: Option<Map<String, Value>>,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Error
    }
}

impl Config {
    pub fn discover(start_dir: &Path) -> Option<Self> {
        let candidates = ["stx.json", ".stylelintrc", ".stylelintrc.json"];

        let mut current = Some(start_dir);
        while let Some(dir) = current {
            for name in candidates {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    if let Ok(cfg) = Self::from_file(&candidate) {
                        return Some(cfg);
                    }
                }
            }
            current = dir.parent();
        }
        None
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::from_json_str(&content, base_dir)
    }

    pub fn from_json_str(content: &str, base_dir: &Path) -> Result<Self, String> {
        let raw: RawConfig = serde_json::from_str(content).map_err(|e| e.to_string())?;
        let mut config = Self::default();

        if let Some(sev) = &raw.default_severity {
            if sev.eq_ignore_ascii_case("warning") {
                config.default_severity = Severity::Warning;
            }
        }

        if let Some(extends) = &raw.extends {
            match extends {
                Value::String(s) => config.apply_extend(s, base_dir),
                Value::Array(arr) => {
                    for item in arr {
                        if let Value::String(s) = item {
                            config.apply_extend(s, base_dir);
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(rules) = raw.rules {
            for (name, val) in rules {
                let parsed = Self::parse_rule_value(val, config.default_severity);
                config.rules.insert(name, parsed);
            }
        }

        Ok(config)
    }

    pub fn standard_preset() -> Self {
        let mut cfg = Self::recommended_preset();
        let kebab = "^([a-z][a-z0-9]*)(-[a-z0-9]+)*$";
        let standard_rules: &[(&str, Value)] = &[
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
            ("container-name-pattern", Value::String(kebab.into())),
            ("custom-media-pattern", Value::String(kebab.into())),
            (
                "custom-property-empty-line-before",
                Value::String("always".into()),
            ),
            ("custom-property-pattern", Value::String(kebab.into())),
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
            ("keyframes-name-pattern", Value::String(kebab.into())),
            ("layer-name-pattern", Value::String(kebab.into())),
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
            ("selector-class-pattern", Value::String(kebab.into())),
            ("selector-id-pattern", Value::String(kebab.into())),
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
                (*name).to_string(),
                Some(RuleSetting {
                    primary: primary.clone(),
                    secondary: None,
                    severity: Severity::Error,
                }),
            );
        }
        cfg
    }

    pub fn recommended_preset() -> Self {
        let mut cfg = Self::default();
        for name in [
            "at-rule-no-unknown",
            "block-no-empty",
            "comment-no-empty",
            "custom-property-no-missing-var-function",
            "no-empty-source",
            "color-no-invalid-hex",
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
            "named-grid-areas-no-invalid",
            "no-descending-specificity",
            "no-duplicate-at-import-rules",
            "no-duplicate-selectors",
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
        ] {
            cfg.enable(name, Value::Bool(true));
        }
        cfg
    }

    fn enable(&mut self, name: &str, primary: Value) {
        self.rules.insert(
            name.to_string(),
            Some(RuleSetting {
                primary,
                secondary: None,
                severity: self.default_severity,
            }),
        );
    }

    fn apply_extend(&mut self, extend_name: &str, base_dir: &Path) {
        let preset = match extend_name {
            "stylelint-config-recommended" | "stx:recommended" => Some(Self::recommended_preset()),
            "stylelint-config-standard" | "stx:standard" => Some(Self::standard_preset()),
            _ => {
                let rel_path: PathBuf = base_dir.join(extend_name);
                if rel_path.is_file() {
                    Self::from_file(&rel_path).ok()
                } else {
                    None
                }
            }
        };

        if let Some(parent) = preset {
            for (k, v) in parent.rules {
                self.rules.insert(k, v);
            }
        }
    }

    fn parse_rule_value(val: Value, default_severity: Severity) -> Option<RuleSetting> {
        match val {
            Value::Null | Value::Bool(false) => None,
            Value::String(ref s) if s == "off" => None,
            Value::Array(mut arr) => {
                if arr.is_empty() {
                    return None;
                }
                let first = arr.remove(0);
                if first.is_null() || first == Value::Bool(false) {
                    return None;
                }
                let secondary = if !arr.is_empty() {
                    match arr.remove(0) {
                        Value::Object(map) => Some(map),
                        _ => None,
                    }
                } else {
                    None
                };

                let mut severity = default_severity;
                if let Some(ref sec) = secondary {
                    if let Some(Value::String(sev_str)) = sec.get("severity") {
                        if sev_str.eq_ignore_ascii_case("warning") {
                            severity = Severity::Warning;
                        } else if sev_str.eq_ignore_ascii_case("error") {
                            severity = Severity::Error;
                        }
                    }
                }

                Some(RuleSetting {
                    primary: first,
                    secondary,
                    severity,
                })
            }
            other => Some(RuleSetting {
                primary: other,
                secondary: None,
                severity: default_severity,
            }),
        }
    }
}

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
        cfg.enable("color-hex-length", Value::String("short".into()));
        cfg
    }

    pub fn recommended_preset() -> Self {
        let mut cfg = Self::default();
        for name in [
            "block-no-empty",
            "comment-no-empty",
            "no-empty-source",
            "color-no-invalid-hex",
            "keyframe-declaration-no-important",
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

mod presets;
mod validate;

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
}

#[cfg(test)]
mod tests {
    use super::Config;
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
}

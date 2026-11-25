use super::{Config, RuleSetting};
use crate::diagnostic::Severity;
use serde_json::Value;

impl Config {
    pub fn recommended_preset() -> Self {
        let mut cfg = Self::default();
        let recommended = [
            "annotation-no-unknown",
            "at-charset-rule-no-invalid",
            "at-rule-descriptor-no-unknown",
            "at-rule-descriptor-value-no-unknown",
            "at-rule-no-deprecated",
            "at-rule-no-unknown",
            "at-rule-prelude-no-invalid",
            "block-no-empty",
            "block-no-redundant-nested-style-rules",
            "comment-no-empty",
            "custom-property-no-missing-var-function",
            "declaration-block-no-duplicate-custom-properties",
            "declaration-block-no-duplicate-properties",
            "declaration-block-no-shorthand-property-overrides",
            "declaration-property-value-keyword-no-deprecated",
            "declaration-property-value-no-unknown",
            "font-family-no-duplicate-names",
            "font-family-no-missing-generic-family-keyword",
            "function-calc-no-unspaced-operator",
            "keyframe-block-no-duplicate-selectors",
            "keyframe-declaration-no-important",
            "media-feature-name-no-unknown",
            "media-feature-name-value-no-unknown",
            "media-query-no-invalid",
            "media-type-no-deprecated",
            "named-grid-areas-no-invalid",
            "nesting-selector-no-missing-scoping-root",
            "no-descending-specificity",
            "no-duplicate-at-import-rules",
            "no-duplicate-selectors",
            "no-empty-source",
            "no-invalid-double-slash-comments",
            "no-invalid-position-at-import-rule",
            "no-invalid-position-declaration",
            "no-irregular-whitespace",
            "property-no-deprecated",
            "property-no-unknown",
            "selector-anb-no-unmatchable",
            "selector-no-deprecated",
            "selector-no-invalid",
            "selector-no-unmatchable",
            "selector-pseudo-class-no-unknown",
            "selector-pseudo-element-no-unknown",
            "selector-type-no-unknown",
            "string-no-newline",
            "syntax-string-no-invalid",
            "value-no-invalid",
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
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::rules::build_rules;

    #[test]
    fn default_preset_checks_unknown_descriptors() {
        assert!(
            build_rules(&Config::standard_preset())
                .iter()
                .any(|configured| { configured.rule.name() == "at-rule-descriptor-no-unknown" })
        );
    }
}

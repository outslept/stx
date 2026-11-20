use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StylelintWarning {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub rule: String,
    pub severity: &'static str,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct StylelintFileResult {
    pub source: String,
    pub errored: bool,
    pub warnings: Vec<StylelintWarning>,
}

pub fn format_human(results: &[StylelintFileResult]) -> String {
    let mut out = String::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;

    for file in results {
        if file.warnings.is_empty() {
            continue;
        }
        out.push_str(&file.source);
        out.push('\n');
        for w in &file.warnings {
            if w.severity == "error" {
                total_errors += 1;
            } else {
                total_warnings += 1;
            }
            out.push_str(&format!(
                "  {}:{}  {}  {}\n",
                w.line, w.column, w.severity, w.text
            ));
        }
        out.push('\n');
    }

    let total = total_errors + total_warnings;
    if total > 0 {
        out.push_str(&format!(
            "{} problems ({} errors, {} warnings)\n",
            total, total_errors, total_warnings
        ));
    }

    out
}

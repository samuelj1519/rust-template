use serde_json::Value;
use std::{
    env, fs,
    path::{Component, Path},
    process::ExitCode,
};

fn counts(report: &Value, metric: &str) -> Result<(u64, u64), String> {
    let data = report
        .get("data")
        .and_then(Value::as_array)
        .filter(|data| data.len() == 1)
        .ok_or_else(|| "Coverage reports must contain exactly one data object".to_owned())?;
    let values = data[0]
        .get("totals")
        .and_then(|totals| totals.get(metric))
        .ok_or_else(|| format!("Coverage report is missing {metric}"))?;
    let count = values
        .get("count")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{metric}.count must be a nonnegative integer"))?;
    let covered = values
        .get("covered")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{metric}.covered must be a nonnegative integer"))?;
    if count == 0 || covered > count {
        return Err(format!(
            "{metric}: missing measurement or invalid counts {covered}/{count}"
        ));
    }
    Ok((count, covered))
}

fn compare(base: &Value, current: &Value) -> Result<(), String> {
    for metric in ["lines", "functions", "regions"] {
        let (before_count, before_covered) = counts(base, metric)?;
        let (after_count, after_covered) = counts(current, metric)?;
        eprintln!("{metric}: {before_covered}/{before_count} -> {after_covered}/{after_count}");
        if u128::from(after_covered) * u128::from(before_count)
            < u128::from(before_covered) * u128::from(after_count)
        {
            return Err(format!(
                "{metric} coverage decreased; restore behavior coverage before retrying"
            ));
        }
    }
    Ok(())
}

fn validate_host(report: &Value, root: &Path) -> Result<(), String> {
    for metric in ["lines", "functions", "regions"] {
        counts(report, metric)?;
    }
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let files = report["data"][0]
        .get("files")
        .and_then(Value::as_array)
        .filter(|files| !files.is_empty())
        .ok_or_else(|| "The test-host report has no source files".to_owned())?;
    for file in files {
        let name = file
            .get("filename")
            .and_then(Value::as_str)
            .ok_or_else(|| "Coverage source entry is missing filename".to_owned())?;
        let source = Path::new(name)
            .canonicalize()
            .map_err(|error| format!("Cannot resolve coverage source {name}: {error}"))?;
        let relative = source.strip_prefix(&root).map_err(|_| {
            format!("The test-host report includes source outside its package: {name}")
        })?;
        let allowed = matches!(relative.components().next(), Some(Component::Normal(name))
            if name == "tests" || name == "examples" || name == "src");
        if !allowed || source.extension().is_none_or(|extension| extension != "rs") {
            return Err(format!(
                "The test-host report includes unrelated source: {name}"
            ));
        }
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let paths: Vec<_> = env::args_os().skip(1).collect();
    let read = |path: &std::ffi::OsStr| -> Result<Value, String> {
        let path = std::path::Path::new(path);
        let data = fs::read(path).map_err(|error| format!("Read {}: {error}", path.display()))?;
        serde_json::from_slice(&data).map_err(|error| format!("Parse {}: {error}", path.display()))
    };
    match paths.as_slice() {
        [flag, root, report] if flag == "--validate-host" => {
            validate_host(&read(report)?, Path::new(root))
        }
        [base, current] => compare(&read(base)?, &read(current)?),
        _ => Err("Usage: compare_coverage <baseline.json> <current.json> or --validate-host <source-root> <report.json>".to_owned()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{compare, validate_host};
    use serde_json::{Value, json};
    use std::path::Path;

    fn report(count: u64, covered: u64) -> Value {
        json!({"data": [{"totals": {
            "lines": {"count": count, "covered": covered},
            "functions": {"count": count, "covered": covered},
            "regions": {"count": count, "covered": covered}
        }}]})
    }

    #[test]
    fn accepts_equal_and_improved_rates() {
        assert!(compare(&report(4, 3), &report(8, 6)).is_ok());
        assert!(compare(&report(4, 3), &report(5, 4)).is_ok());
    }

    #[test]
    fn rejects_drop_hidden_by_display_rounding() {
        assert!(compare(&report(1_000_000, 999_999), &report(999_999, 999_998)).is_err());
    }

    #[test]
    fn checks_each_required_metric() {
        for metric in ["lines", "functions", "regions"] {
            let mut current = report(4, 4);
            current["data"][0]["totals"][metric]["covered"] = json!(3);
            assert!(compare(&report(4, 4), &current).is_err());
        }
    }

    #[test]
    fn rejects_empty_malformed_and_impossible_reports() {
        assert!(compare(&report(0, 0), &report(1, 1)).is_err());
        assert!(compare(&report(1, 1), &report(0, 0)).is_err());
        assert!(compare(&report(4, 5), &report(4, 4)).is_err());
        assert!(compare(&json!({}), &report(4, 4)).is_err());
        let mut decimal = report(4, 4);
        decimal["data"][0]["totals"]["lines"]["count"] = json!(4.5);
        assert!(compare(&decimal, &report(4, 4)).is_err());
    }

    #[test]
    fn rejects_ambiguous_multiple_data_entries() {
        let mut ambiguous = report(4, 4);
        ambiguous["data"] = json!([report(4, 4)["data"][0], report(4, 4)["data"][0]]);
        assert!(compare(&ambiguous, &report(4, 4)).is_err());
    }

    #[test]
    fn compares_large_counters_without_overflow() {
        assert!(compare(&report(u64::MAX, u64::MAX - 1), &report(u64::MAX, u64::MAX)).is_ok());
        assert!(compare(&report(u64::MAX, u64::MAX), &report(u64::MAX, u64::MAX - 1)).is_err());
    }

    #[test]
    fn validates_nonempty_host_scope_without_external_sources() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut data = report(4, 4);
        data["data"][0]["files"] = json!([
            {"filename": root.join("tests/template_generation.rs")},
            {"filename": root.join("examples/compare_coverage.rs")}
        ]);
        assert!(validate_host(&data, root).is_ok());
        data["data"][0]["files"] = json!([]);
        assert!(validate_host(&data, root).is_err());
        data["data"][0]["files"] = json!([{"filename": root.join("Cargo.toml")}]);
        assert!(validate_host(&data, root).is_err());
        let outside = std::env::temp_dir().join(format!(
            "rust-template-coverage-outside-{}.rs",
            std::process::id()
        ));
        std::fs::write(&outside, "pub fn outside() {}\n").unwrap();
        data["data"][0]["files"] = json!([{"filename": outside}]);
        let result = validate_host(&data, root);
        std::fs::remove_file(outside).unwrap();
        assert!(result.unwrap_err().contains("outside"));
    }
}

use anyhow::{Context, Result};
use rquickjs::{Context as JsContext, Function, Module, Object, Runtime, Value};

/// Run `script` (an ES module with a default-export function) on `json_str`.
/// Returns the transformed value as a `serde_json::Value`.
pub fn run_transform(json_str: &str, script: &str) -> Result<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(json_str).context("Invalid JSON input")?;

    let rt = Runtime::new().context("Failed to create JS runtime")?;
    let ctx = JsContext::full(&rt).context("Failed to create JS context")?;

    let result_str = ctx
        .with(|ctx| -> rquickjs::Result<String> {
            let module = Module::declare(ctx.clone(), "transform", script)?;
            let (module, _promise) = module.eval()?;
            let ns: Object = module.namespace()?;
            let func: Function = ns.get("default")?;

            let globals = ctx.globals();
            let json_obj: Object = globals.get("JSON")?;
            let parse: Function = json_obj.get("parse")?;
            let js_input: Value = parse.call((json_str,))?;

            let result: Value = func.call((js_input,))?;

            let stringify: Function = json_obj.get("stringify")?;
            stringify.call((result,))
        })
        .map_err(|e| anyhow::anyhow!("Script execution failed: {e}"))?;

    serde_json::from_str(&result_str).context("Script returned a non-JSON-serialisable value")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const IDENTITY_SCRIPT: &str = r#"export default function(data) { return data; }"#;

    #[test]
    fn passthrough_preserves_input() {
        let input = json!({"name": "Alice", "age": 30});
        let result = run_transform(&input.to_string(), IDENTITY_SCRIPT).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn transform_adds_field() {
        let script = r#"export default function(data) { return { ...data, ok: true }; }"#;
        let result = run_transform(r#"{"x": 1}"#, script).unwrap();
        assert_eq!(result["x"], 1);
        assert_eq!(result["ok"], true);
    }

    #[test]
    fn transform_filters_array() {
        let script = r#"
            export default function(data) {
                return data.filter(n => n > 2);
            }
        "#;
        let result = run_transform("[1, 2, 3, 4]", script).unwrap();
        assert_eq!(result, json!([3, 4]));
    }

    #[test]
    fn sample_script_matches_expected_shape() {
        let json_str = std::fs::read_to_string("examples/sample.json").unwrap();
        let script = std::fs::read_to_string("examples/sample.js").unwrap();
        let result = run_transform(&json_str, &script).unwrap();
        assert_eq!(result["version"], "1.0");
        assert_eq!(result["total_users"], 2);
        assert_eq!(result["admins"], json!(["Alice"]));
        assert!(result["processed_at"].is_string());
    }

    #[test]
    fn invalid_json_returns_error() {
        let err = run_transform("not json", IDENTITY_SCRIPT).unwrap_err();
        assert!(err.to_string().contains("Invalid JSON"));
    }

    #[test]
    fn script_without_default_export_returns_error() {
        let script = r#"export function notDefault(data) { return data; }"#;
        let err = run_transform("{}", script).unwrap_err();
        assert!(err.to_string().contains("Script execution failed"));
    }

    #[test]
    fn script_that_throws_returns_error() {
        let script = r#"export default function(_) { throw new Error("boom"); }"#;
        let err = run_transform("{}", script).unwrap_err();
        assert!(err.to_string().contains("Script execution failed"));
    }

    #[test]
    fn script_returning_null_is_valid() {
        let script = r#"export default function(_) { return null; }"#;
        let result = run_transform("{}", script).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn script_returning_undefined_is_error() {
        // JSON.stringify(undefined) → undefined (not a string), so the Rust
        // side cannot parse it — expect an error.
        let script = r#"export default function(_) { return undefined; }"#;
        let err = run_transform("{}", script).unwrap_err();
        assert!(err.to_string().contains("Script execution failed")
            || err.to_string().contains("non-JSON"));
    }
}

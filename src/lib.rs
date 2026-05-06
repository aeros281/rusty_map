use anyhow::{Context, Result};
use rquickjs::{Context as JsContext, Function, Module, Object, Runtime, Value};

// JS orchestrator: receives each optional pipeline fn (or undefined) + the parsed input.
// before_transform -> try_filter/try_map (per item, array only) -> after_transform.
const PIPELINE_JS: &str = r#"(function(before, filter, map, after, input) {
    var transformCtx = (typeof before === 'function') ? before() : {};
    var processed = input;
    if (typeof filter === 'function' || typeof map === 'function') {
        var wasWrapped = !Array.isArray(input);
        var arr = wasWrapped ? [input] : input;
        processed = arr
            .filter(function(item) {
                return typeof filter === 'function' ? filter(item, transformCtx) : true;
            })
            .map(function(item) {
                return typeof map === 'function' ? map(item, transformCtx) : item;
            });
        if (wasWrapped) {
            processed = processed.length > 0 ? processed[0] : null;
        }
    }
    return (typeof after === 'function') ? after(processed) : processed;
})"#;

/// Run `script` (an ES module) on `json_str` using the pipeline:
/// `before_transform` → `try_filter` / `try_map` (per item) → `after_transform`.
/// All four exports are optional.
pub fn run_transform(json_str: &str, script: &str) -> Result<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(json_str).context("Invalid JSON input")?;

    let rt = Runtime::new().context("Failed to create JS runtime")?;
    let ctx = JsContext::full(&rt).context("Failed to create JS context")?;

    let result_str = ctx
        .with(|ctx| -> rquickjs::Result<String> {
            let module = Module::declare(ctx.clone(), "transform", script)?;
            let (module, _promise) = module.eval()?;
            let ns: Object = module.namespace()?;

            let globals = ctx.globals();
            let json_obj: Object = globals.get("JSON")?;
            let parse: Function = json_obj.get("parse")?;
            let stringify: Function = json_obj.get("stringify")?;

            let js_input: Value = parse.call((json_str,))?;

            // Fetch each export as Value (undefined when absent) so the JS
            // orchestrator can do typeof-checks rather than us poking at the
            // module namespace exotic object from Rust.
            let before_val: Value = ns.get("before_transform")?;
            let filter_val: Value = ns.get("try_filter")?;
            let map_val: Value = ns.get("try_map")?;
            let after_val: Value = ns.get("after_transform")?;

            let pipeline_fn: Function = ctx.eval(PIPELINE_JS.as_bytes())?;
            let final_result: Value =
                pipeline_fn.call((before_val, filter_val, map_val, after_val, js_input))?;

            stringify.call((final_result,))
        })
        .map_err(|e| anyhow::anyhow!("Script execution failed: {e}"))?;

    serde_json::from_str(&result_str).context("Script returned a non-JSON-serialisable value")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_pipeline_fns_passes_through() {
        let script = r#"export const placeholder = true;"#;
        let input = json!({"name": "Alice", "age": 30});
        let result = run_transform(&input.to_string(), script).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn after_transform_adds_field() {
        let script = r#"export function after_transform(data) { return { ...data, ok: true }; }"#;
        let result = run_transform(r#"{"x": 1}"#, script).unwrap();
        assert_eq!(result["x"], 1);
        assert_eq!(result["ok"], true);
    }

    #[test]
    fn try_filter_removes_items() {
        let script = r#"export function try_filter(item) { return item > 2; }"#;
        let result = run_transform("[1, 2, 3, 4]", script).unwrap();
        assert_eq!(result, json!([3, 4]));
    }

    #[test]
    fn try_map_transforms_items() {
        let script = r#"export function try_map(item) { return item * 2; }"#;
        let result = run_transform("[1, 2, 3]", script).unwrap();
        assert_eq!(result, json!([2, 4, 6]));
    }

    #[test]
    fn before_transform_context_passed_to_filter_and_map() {
        let script = r#"
            export function before_transform() { return { threshold: 2, multiplier: 10 }; }
            export function try_filter(item, ctx) { return item > ctx.threshold; }
            export function try_map(item, ctx) { return item * ctx.multiplier; }
        "#;
        let result = run_transform("[1, 2, 3, 4]", script).unwrap();
        assert_eq!(result, json!([30, 40]));
    }

    #[test]
    fn full_pipeline_with_all_four_fns() {
        let script = r#"
            export function before_transform() { return { min: 1 }; }
            export function try_filter(item, ctx) { return item.value > ctx.min; }
            export function try_map(item) { return item.value; }
            export function after_transform(items) { return { total: items.length, items }; }
        "#;
        let result = run_transform(
            r#"[{"value": 1}, {"value": 2}, {"value": 3}]"#,
            script,
        )
        .unwrap();
        assert_eq!(result["total"], 2);
        assert_eq!(result["items"], json!([2, 3]));
    }

    #[test]
    fn try_map_applied_to_object_via_wrapping() {
        let script = r#"export function try_map(item) { return { ...item, added: true }; }"#;
        let input = json!({"key": "value"});
        let result = run_transform(&input.to_string(), script).unwrap();
        assert_eq!(result, json!({"key": "value", "added": true}));
    }

    #[test]
    fn try_filter_false_on_object_returns_null() {
        let script = r#"export function try_filter(_) { return false; }"#;
        let input = json!({"key": "value"});
        let result = run_transform(&input.to_string(), script).unwrap();
        assert!(result.is_null());
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
        let script = r#"export const x = 1;"#;
        let err = run_transform("not json", script).unwrap_err();
        assert!(err.to_string().contains("Invalid JSON"));
    }

    #[test]
    fn after_transform_that_throws_returns_error() {
        let script = r#"export function after_transform(_) { throw new Error("boom"); }"#;
        let err = run_transform("{}", script).unwrap_err();
        assert!(err.to_string().contains("Script execution failed"));
    }

    #[test]
    fn before_transform_that_throws_returns_error() {
        let script = r#"export function before_transform() { throw new Error("setup failed"); }"#;
        let err = run_transform("[1, 2]", script).unwrap_err();
        assert!(err.to_string().contains("Script execution failed"));
    }

    #[test]
    fn try_filter_that_throws_returns_error() {
        let script = r#"export function try_filter(_) { throw new Error("filter failed"); }"#;
        let err = run_transform("[1, 2]", script).unwrap_err();
        assert!(err.to_string().contains("Script execution failed"));
    }

    #[test]
    fn try_map_that_throws_returns_error() {
        let script = r#"export function try_map(_) { throw new Error("map failed"); }"#;
        let err = run_transform("[1, 2]", script).unwrap_err();
        assert!(err.to_string().contains("Script execution failed"));
    }

    #[test]
    fn after_transform_returning_null_is_valid() {
        let script = r#"export function after_transform(_) { return null; }"#;
        let result = run_transform("{}", script).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn after_transform_returning_undefined_is_error() {
        let script = r#"export function after_transform(_) { return undefined; }"#;
        let err = run_transform("{}", script).unwrap_err();
        assert!(
            err.to_string().contains("Script execution failed")
                || err.to_string().contains("non-JSON")
        );
    }
}

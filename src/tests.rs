use super::*;
use serde_json::json;

use httpmock::prelude::*;

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

// --- rb_read_file ---

#[test]
fn rb_read_file_reads_existing_file() {
    let path = std::env::temp_dir().join("rusty_map_test_read.txt");
    std::fs::write(&path, "hello world").unwrap();
    let script = format!(
        r#"export function after_transform(_) {{ return rb_read_file("{}"); }}"#,
        path.display()
    );
    let result = run_transform("{}", &script).unwrap();
    assert_eq!(result, json!("hello world"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn rb_read_file_missing_file_throws_js_error() {
    let script = r#"
        export function after_transform(_) {
            try { rb_read_file("/no/such/file_rusty_map.txt"); return "ok"; }
            catch(e) { return "caught: " + e.message; }
        }
    "#;
    let result = run_transform("{}", script).unwrap();
    assert!(result.as_str().unwrap().starts_with("caught:"));
}

#[test]
fn rb_read_file_result_usable_in_pipeline() {
    let path = std::env::temp_dir().join("rusty_map_test_pipeline.json");
    std::fs::write(&path, r#"{"score": 42}"#).unwrap();
    let script = format!(
        r#"export function after_transform(_) {{
            var raw = rb_read_file("{}");
            var obj = JSON.parse(raw);
            return obj.score;
        }}"#,
        path.display()
    );
    let result = run_transform("{}", &script).unwrap();
    assert_eq!(result, json!(42));
    std::fs::remove_file(&path).ok();
}

// --- rb_http_get ---

#[test]
fn rb_http_get_returns_response_body() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).body("pong");
    });
    let script = format!(
        r#"export function after_transform(_) {{ return rb_http_get("{}", null, null); }}"#,
        server.url("/")
    );
    let result = run_transform("{}", &script).unwrap();
    assert_eq!(result, json!("pong"));
}

#[test]
fn rb_http_get_sends_custom_headers() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/")
            .header("Authorization", "Bearer token")
            .header("X-Custom", "value");
        then.status(200).body("authorized");
    });
    let script = format!(
        r#"export function after_transform(_) {{
            return rb_http_get("{}", null, {{"Authorization": "Bearer token", "X-Custom": "value"}});
        }}"#,
        server.url("/")
    );
    let result = run_transform("{}", &script).unwrap();
    assert_eq!(result, json!("authorized"));
}

#[test]
fn rb_http_get_invalid_url_throws_js_error() {
    let script = r#"
        export function after_transform(_) {
            try { rb_http_get("http://127.0.0.1:1/"); return "ok"; }
            catch(e) { return "caught"; }
        }
    "#;
    let result = run_transform("{}", script).unwrap();
    assert_eq!(result, json!("caught"));
}

// --- rb_http_post ---

#[test]
fn rb_http_post_sends_body_and_returns_response() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST)
            .path("/")
            .header("Content-Type", "application/json")
            .body(r#"{"key":"val"}"#);
        then.status(201).body("created");
    });
    let script = format!(
        r#"export function after_transform(_) {{
            return rb_http_post(
                "{}",
                JSON.stringify({{key: "val"}}),
                {{"Content-Type": "application/json"}}
            );
        }}"#,
        server.url("/")
    );
    let result = run_transform("{}", &script).unwrap();
    assert_eq!(result, json!("created"));
}

#[test]
fn rb_http_post_no_body_no_headers() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200).body("bare");
    });
    let script = format!(
        r#"export function after_transform(_) {{ return rb_http_post("{}", null, null); }}"#,
        server.url("/")
    );
    let result = run_transform("{}", &script).unwrap();
    assert_eq!(result, json!("bare"));
}

#[test]
fn rb_http_post_sends_auth_header() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/").header("Authorization", "Bearer secret");
        then.status(200).body("ok");
    });
    let script = format!(
        r#"export function after_transform(_) {{
            return rb_http_post("{}", null, {{"Authorization": "Bearer secret"}});
        }}"#,
        server.url("/")
    );
    let result = run_transform("{}", &script).unwrap();
    assert_eq!(result, json!("ok"));
}

#[test]
fn rb_http_post_invalid_url_throws_js_error() {
    let script = r#"
        export function after_transform(_) {
            try { rb_http_post("http://127.0.0.1:1/", null, null); return "ok"; }
            catch(e) { return "caught"; }
        }
    "#;
    let result = run_transform("{}", script).unwrap();
    assert_eq!(result, json!("caught"));
}

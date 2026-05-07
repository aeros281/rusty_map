use anyhow::{Context, Result};
use rquickjs::{Context as JsContext, Ctx, Exception, Function, Module, Object, Runtime, Value};

// JS orchestrator: parses JSON input, runs the pipeline, and returns pretty-printed JSON.
// before_transform -> try_filter/try_map (per item, array only) -> after_transform.
const PIPELINE_JS: &str = r#"(function(before, filter, map, after, jsonStr) {
    var input;
    try { input = JSON.parse(jsonStr); }
    catch(e) { throw new Error("Invalid JSON input: " + e.message); }
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
    var result = (typeof after === 'function') ? after(processed) : processed;
    var out = JSON.stringify(result, null, 2);
    if (typeof out === 'undefined') { throw new Error("Script returned a non-JSON-serialisable value"); }
    return out;
})"#;

fn rb_read_file<'js>(ctx: Ctx<'js>, path: String) -> rquickjs::Result<String> {
    std::fs::read_to_string(&path).map_err(|e| {
        ctx.throw(Exception::from_message(ctx.clone(), &e.to_string()).expect("OOM").into())
    })
}

// headers may be a plain JS object, null, or undefined — all are safe to pass.
fn apply_headers<'js>(
    mut builder: reqwest::blocking::RequestBuilder,
    headers: Value<'js>,
) -> rquickjs::Result<reqwest::blocking::RequestBuilder> {
    if let Some(obj) = headers.into_object() {
        for entry in obj.props::<String, String>() {
            let (k, v) = entry?;
            builder = builder.header(k, v);
        }
    }
    Ok(builder)
}

// rb_http_get(url, body?, headers?)
fn rb_http_get<'js>(
    ctx: Ctx<'js>,
    url: String,
    body: Option<String>,
    headers: Value<'js>,
) -> rquickjs::Result<String> {
    let client = reqwest::blocking::Client::new();
    let mut builder = apply_headers(client.get(&url), headers)?;
    if let Some(b) = body {
        builder = builder.body(b);
    }
    builder
        .send()
        .and_then(|r| r.text())
        .map_err(|e| {
            ctx.throw(Exception::from_message(ctx.clone(), &e.to_string()).expect("OOM").into())
        })
}

// rb_http_post(url, body?, headers?)
fn rb_http_post<'js>(
    ctx: Ctx<'js>,
    url: String,
    body: Option<String>,
    headers: Value<'js>,
) -> rquickjs::Result<String> {
    let client = reqwest::blocking::Client::new();
    let mut builder = apply_headers(client.post(&url), headers)?;
    if let Some(b) = body {
        builder = builder.body(b);
    }
    builder
        .send()
        .and_then(|r| r.text())
        .map_err(|e| {
            ctx.throw(Exception::from_message(ctx.clone(), &e.to_string()).expect("OOM").into())
        })
}

fn register_bindings<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    globals.set("rb_read_file", Function::new(ctx.clone(), rb_read_file)?)?;
    globals.set("rb_http_get", Function::new(ctx.clone(), rb_http_get)?)?;
    globals.set("rb_http_post", Function::new(ctx.clone(), rb_http_post)?)?;
    Ok(())
}

fn format_js_error(ctx: &Ctx<'_>, e: rquickjs::Error) -> anyhow::Error {
    if matches!(e, rquickjs::Error::Exception) {
        let caught = ctx.catch();
        if let Some(exc) = caught.as_exception() {
            let detail = exc
                .message()
                .or_else(|| exc.stack())
                .unwrap_or_else(|| "unknown exception".into());
            return anyhow::anyhow!("Script execution failed: {}", detail);
        }
        if let Some(s) = caught.as_string().and_then(|js| js.to_string().ok()) {
            return anyhow::anyhow!("Script execution failed: {}", s);
        }
    }
    anyhow::anyhow!("Script execution failed: {}", e)
}

/// Run `script` (an ES module) on `json_str` using the pipeline:
/// `before_transform` → `try_filter` / `try_map` (per item) → `after_transform`.
/// All four exports are optional. Returns pretty-printed JSON.
pub fn run_transform(json_str: &str, script: &str) -> Result<String> {
    let rt = Runtime::new().context("Failed to create JS runtime")?;
    let ctx = JsContext::full(&rt).context("Failed to create JS context")?;

    ctx.with(|ctx| -> Result<String> {
        let run = || -> rquickjs::Result<String> {
            register_bindings(&ctx)?;

            let module = Module::declare(ctx.clone(), "transform", script)?;
            let (module, _promise) = module.eval()?;
            let ns: Object = module.namespace()?;

            // Fetch each export as Value (undefined when absent) so the JS
            // orchestrator can do typeof-checks rather than us poking at the
            // module namespace exotic object from Rust.
            let before_val: Value = ns.get("before_transform")?;
            let filter_val: Value = ns.get("try_filter")?;
            let map_val: Value = ns.get("try_map")?;
            let after_val: Value = ns.get("after_transform")?;

            let pipeline_fn: Function = ctx.eval(PIPELINE_JS.as_bytes())?;
            pipeline_fn.call((before_val, filter_val, map_val, after_val, json_str))
        };
        run().map_err(|e| format_js_error(&ctx, e))
    })
}

#[cfg(test)]
mod tests;

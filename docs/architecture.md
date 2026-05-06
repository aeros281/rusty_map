# Architecture

## Runtime: rquickjs

[rquickjs](https://github.com/DelSkayn/rquickjs) embeds the QuickJS JavaScript engine as a Rust library.  
QuickJS is a small, self-contained JS engine that supports ES2020 (including ES modules and `async/await`) with no external binary dependencies.

Key call chain in `src/lib.rs`:

```
Runtime::new()                        — create an isolated QuickJS runtime
JsContext::full(&rt)                  — create a context with the full JS standard library
Module::declare(ctx, name, src)       — compile the script as an ES module
module.eval()                         — evaluate the module, resolving the module promise
module.namespace()                    — retrieve the module's namespace object
ns.get("before_transform")            — fetch export as Value (undefined if absent)
ns.get("try_filter")                  —   "
ns.get("try_map")                     —   "
ns.get("after_transform")             —   "
JSON.parse(json_str)                  — deserialise the input inside the JS engine
ctx.eval(PIPELINE_JS)                 — compile the JS orchestrator function
pipeline_fn.call((b, f, m, a, input)) — run the full pipeline
JSON.stringify(final_result)          — serialise the return value back to a string
serde_json::from_str(&result_str)     — hand the JSON string to Rust's serde_json
```

Each of the four exports is fetched as a `Value` (returns `undefined` when the export is absent) and passed as an argument to `PIPELINE_JS`, a small inline JS function that performs the `typeof`-checks, array iteration, and sequencing entirely in JavaScript.

Input round-trips through `JSON.parse` / `JSON.stringify` inside the JS context so the script receives a live JS object and returns a plain JSON-serialisable value.

## JSON serialisation

`serde_json::Value` is used on the Rust side; the JSON string is parsed and stringified by the engine's own `JSON` global so no manual type mapping is required.  
The script author is responsible for returning a JSON-serialisable value. Returning `undefined` causes `JSON.stringify` to produce a non-string result, which surfaces as a deserialisation error on the Rust side.

## Error handling

All errors bubble up through `anyhow::Result` with contextual messages.  
No partial output is written on error — the process exits non-zero.

Error sources and messages:

| Source | Message prefix |
|--------|----------------|
| Malformed input JSON | `"Invalid JSON input"` |
| QuickJS runtime init | `"Failed to create JS runtime"` |
| Script compile / eval | `"Script execution failed: …"` |
| Non-serialisable return | `"Script returned a non-JSON-serialisable value"` |

## Extension points

| Need | How |
|------|-----|
| Shared setup state across filter / map | Return it from `before_transform`; it arrives as `ctx` in both callbacks |
| Transform a non-array value (object) | `try_filter` / `try_map` wrap the object in a single-element array, apply the callbacks, then unwrap back to a scalar. A filtered-out object becomes `null`. Use `after_transform` alone when no per-item logic is needed. |
| Chain multiple scripts | Feed the `serde_json::Value` output of one `run_transform` call as input to the next |
| TypeScript support | Pre-transpile with `swc` or `esbuild` before passing the source to `Module::declare` |
| Async transforms | Use `rquickjs`'s `Promise` support and drive the runtime event loop with `rt.run_gc()` / `ctx.execute_pending_job()` |

## Dependency choices

| Crate | Reason |
|-------|--------|
| `rquickjs` | Embeds QuickJS; no external binary required, small footprint, ES module support |
| `clap` (derive) | Ergonomic CLI parsing with auto-generated `--help` |
| `serde_json` | Standard JSON in Rust; `Value` type handles schema-free data |
| `anyhow` | Chainable error context without boilerplate |

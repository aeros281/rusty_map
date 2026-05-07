# Architecture

## Runtime: rquickjs

[rquickjs](https://github.com/DelSkayn/rquickjs) embeds the QuickJS JavaScript engine as a Rust library.  
QuickJS is a small, self-contained JS engine that supports ES2020 (including ES modules and `async/await`) with no external binary dependencies.

Key call chain in `src/lib.rs`:

```
Runtime::new()                          — create an isolated QuickJS runtime
JsContext::full(&rt)                    — create a context with the full JS standard library
Module::declare(ctx, name, src)         — compile the script as an ES module
module.eval()                           — evaluate the module, resolving the module promise
module.namespace()                      — retrieve the module's namespace object
ns.get("before_transform")              — fetch export as Value (undefined if absent)
ns.get("try_filter")                    —   "
ns.get("try_map")                       —   "
ns.get("after_transform")               —   "
ctx.eval(PIPELINE_JS)                   — compile the JS orchestrator function
pipeline_fn.call((b, f, m, a, jsonStr)) — run the full pipeline; returns pretty-printed JSON string
```

Each of the four exports is fetched as a `Value` (returns `undefined` when the export is absent) and passed as an argument to `PIPELINE_JS`, a small inline JS function that performs the `typeof`-checks, array iteration, and sequencing entirely in JavaScript.

The raw JSON string is passed directly to `PIPELINE_JS`, which calls `JSON.parse` to deserialise the input and `JSON.stringify(result, null, 2)` to produce the pretty-printed output. All JSON handling stays inside the JS engine — no Rust-side JSON library is involved at runtime.

## Rust bindings (`rb_*` globals)

QuickJS has no built-in I/O or networking. To compensate, `register_bindings` (called once per `run_transform` invocation, before user-script evaluation) registers three native functions as globals on the JS context:

| Global | Rust implementation |
|--------|---------------------|
| `rb_read_file(path)` | `std::fs::read_to_string` |
| `rb_http_get(url, body?, headers?)` | `reqwest::blocking::Client::get` |
| `rb_http_post(url, body?, headers?)` | `reqwest::blocking::Client::post` |

Each function receives `Ctx<'js>` as its first Rust argument so it can convert Rust errors into proper JS `Error` objects via `ctx.throw(Exception::from_message(...))`. This lets scripts use idiomatic `try/catch`.

Headers are accepted as a `Option<Object<'js>>` and forwarded to reqwest via `Object::props::<String, String>()`.

HTTP calls use `reqwest::blocking`, which blocks the calling thread until a response arrives. This integrates cleanly with QuickJS's synchronous execution model.

Call chain addition:
```
register_bindings(&ctx)              — expose rb_read_file, rb_http_get, rb_http_post as globals
Module::declare(ctx, name, src)      — compile the user script (globals already visible)
```

## JSON serialisation

All JSON parsing and serialisation is handled inside the QuickJS engine via its built-in `JSON` global — no Rust JSON library is used at runtime. `run_transform` returns the final output as a `String` (pretty-printed, 2-space indent) produced by `JSON.stringify(result, null, 2)`.

The script author is responsible for returning a JSON-serialisable value. Returning `undefined` causes `JSON.stringify` to produce `undefined` (not a string), which `PIPELINE_JS` detects and converts into a thrown error.

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
| Chain multiple scripts | Feed the `String` output of one `run_transform` call as `json_str` input to the next |
| TypeScript support | Pre-transpile with `swc` or `esbuild` before passing the source to `Module::declare` |
| Async transforms | Use `rquickjs`'s `Promise` support and drive the runtime event loop with `rt.run_gc()` / `ctx.execute_pending_job()` |

## Dependency choices

| Crate | Reason |
|-------|--------|
| `rquickjs` | Embeds QuickJS; no external binary required, small footprint, ES module support |
| `clap` (derive) | Ergonomic CLI parsing with auto-generated `--help` |
| `anyhow` | Chainable error context without boilerplate |
| `reqwest` (blocking) | Synchronous HTTP client for `rb_http_get` / `rb_http_post`; blocking feature avoids introducing an async runtime |

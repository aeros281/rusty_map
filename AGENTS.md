# rusty_map

A Rust CLI that transforms JSON by executing a user-supplied JavaScript script.

## What it does

```
# Read JSON from a file via flag
rusty_map -f <json_file> <script_file>

# Pipe JSON via stdin
cat data.json | rusty_map <script_file>

# Print a starter script template to stdout
rusty_map generate-template
```

1. Reads JSON from `-f <json_file>` or from stdin when the flag is omitted.
2. Loads `script_file` into an embedded QuickJS runtime ([rquickjs](https://github.com/DelSkayn/rquickjs)).
3. Runs the script's pipeline exports against the parsed JSON.
4. Pretty-prints the returned JSON to stdout.

`generate-template` is a standalone subcommand — it prints a ready-to-edit JS file with `try_map` active and the other three hooks commented out.

## Script contract

The script may export any combination of these four named functions (all optional):

| Export | Signature | Purpose |
|--------|-----------|---------|
| `before_transform` | `() → object` | Runs once before iteration; the returned object is passed as `ctx` to `try_filter` and `try_map`. Defaults to `{}` when absent. |
| `try_filter` | `(item, ctx) → boolean` | Called for each item; falsy return drops the item. When input is an object it is temporarily wrapped in a single-element array and unwrapped afterward (filtered-out object becomes `null`). |
| `try_map` | `(item, ctx) → any` | Called for each surviving item; return value replaces the item. Same wrapping behaviour as `try_filter` for object input. |
| `after_transform` | `(result) → any` | Receives the processed value and returns the final output. |

Pipeline order: `before_transform` → `try_filter` / `try_map` (per item) → `after_transform`.  
If none are defined the input passes through unchanged.

```js
export function before_transform() { return { min: 2 }; }
export function try_filter(item, ctx) { return item > ctx.min; }
export function try_map(item) { return item * 10; }
export function after_transform(items) { return { results: items }; }
```

See [examples/sample.js](examples/sample.js) for a working example.

## Built-in JS globals (Rust bindings)

These functions are available in every script without any import. They are implemented in Rust and exposed to QuickJS at startup. All errors are thrown as JS `Error` objects so scripts can `try/catch` them.

| Global | Signature | Purpose |
|--------|-----------|---------|
| `rb_read_file` | `(path: string) → string` | Reads a file from disk; returns the full content as a string. |
| `rb_http_get` | `(url: string, body?: string, headers?: object) → string` | Sends an HTTP GET request; returns the response body as a string. |
| `rb_http_post` | `(url: string, body?: string, headers?: object) → string` | Sends an HTTP POST request; returns the response body as a string. |

Pass headers as a plain object: `{ "Authorization": "Bearer token", "Content-Type": "application/json" }`.

```js
export function after_transform(items) {
    // enrich each item with data from a sidecar file
    const config = JSON.parse(rb_read_file("/etc/my-tool/config.json"));
    // call an external API with auth
    const raw = rb_http_post(
        "https://api.example.com/enrich",
        JSON.stringify({ ids: items }),
        { "Authorization": "Bearer " + config.token, "Content-Type": "application/json" }
    );
    return JSON.parse(raw);
}
```

## Build & run

```sh
cargo build --release

# file flag
./target/release/rusty_map -f examples/sample.json examples/sample.js

# stdin pipe
cat examples/sample.json | ./target/release/rusty_map examples/sample.js

# scaffold a new script
./target/release/rusty_map generate-template > my_transform.js
```

### Installing with `cargo install`

`cargo install` builds in release mode (optimized) but retains debug symbols by default. To produce a stripped binary, set `strip = true` in `Cargo.toml`:

```toml
[profile.release]
strip = true
```

Then install:

```sh
cargo install --path .
```

The resulting binary in `~/.cargo/bin/` will be optimized and symbol-free.

## Key files

| Path | Purpose |
|------|---------|
| `src/main.rs` | CLI entry-point |
| `src/lib.rs` | Pipeline logic and JS orchestration |
| `examples/sample.json` | Sample input |
| `examples/sample.js` | Sample transform script |
| `docs/architecture.md` | Design decisions and extension points |
| `docs/commit-conventions.md` | Commit message prefix conventions |

## Updating this file

AGENTS.md is the **entry-point contract** for agents and developers.  
Keep it short: what the tool does, the script interface, and pointers to deeper docs.

Update it when:
- The CLI interface (flags / arguments) changes.
- The script contract changes (function signature, module format).
- A new key file is added or removed.

Do **not** put build troubleshooting, dependency rationale, or design notes here — those go in [docs/architecture.md](docs/architecture.md).

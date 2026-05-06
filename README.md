# rusty_map

A Rust CLI that transforms JSON by executing a user-supplied JavaScript script.

## How it works

```sh
# Pass a JSON file via flag
rusty_map -f <json_file> <script_file>

# Pipe JSON via stdin
cat data.json | rusty_map <script_file>
```

1. Reads JSON from `-f <json_file>` or from stdin when the flag is omitted.
2. Loads `script_file` into an embedded [QuickJS](https://github.com/DelSkayn/rquickjs) runtime.
3. Runs the script's pipeline exports against the parsed JSON.
4. Pretty-prints the returned value to stdout.

## Script contract

The script may export any combination of these four named functions (all optional):

| Export | Signature | Purpose |
|--------|-----------|---------|
| `before_transform` | `() → object` | Runs once before iteration; the returned object is passed as `ctx` to `try_filter` and `try_map`. Defaults to `{}` when absent. |
| `try_filter` | `(item, ctx) → boolean` | Called for each item; falsy return drops the item. When input is an object it is temporarily wrapped in an array, then unwrapped afterward. |
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

### Example

**`examples/sample.json`**
```json
[
  { "id": 1, "name": "Alice", "role": "admin",  "score": 90 },
  { "id": 2, "name": "Bob",   "role": "viewer", "score": 40 },
  { "id": 3, "name": "Carol", "role": "editor", "score": 75 }
]
```

**`examples/sample.js`**
```js
export function before_transform() {
  return { minScore: 50 };
}

export function try_filter(item, ctx) {
  return item.score >= ctx.minScore;
}

export function try_map(item) {
  return {
    ...item,
    display_name: `${item.role.charAt(0).toUpperCase() + item.role.slice(1)}: ${item.name}`,
  };
}

export function after_transform(items) {
  return {
    version: "1.0",
    total_users: items.length,
    admins: items.filter(u => u.role === "admin").map(u => u.name),
    processed_at: new Date().toISOString(),
  };
}
```

**Output**
```json
{
  "version": "1.0",
  "total_users": 2,
  "admins": ["Alice"],
  "processed_at": "2026-05-06T00:00:00.000Z"
}
```

## Installation

### From GitHub

```sh
cargo install --git https://github.com/aeros281/rusty_map.git
```

This compiles and installs the `rusty_map` binary to `~/.cargo/bin/`.

### From local source

```sh
git clone https://github.com/aeros281/rusty_map.git
cd rusty_map
cargo install --path .
```

### Build without installing

```sh
cargo build --release

# file flag
./target/release/rusty_map -f examples/sample.json examples/sample.js

# stdin pipe
cat examples/sample.json | ./target/release/rusty_map examples/sample.js
```

## Built-in JS globals

These functions are available in every script without any import. They are implemented in Rust and exposed to QuickJS at startup. All errors are thrown as JS `Error` objects so scripts can `try/catch` them.

| Global | Signature | Purpose |
|--------|-----------|---------|
| `rb_read_file` | `(path: string) → string` | Reads a file from disk; returns the full content as a string. |
| `rb_http_get` | `(url: string, body?: string, headers?: object) → string` | Sends an HTTP GET request; returns the response body as a string. |
| `rb_http_post` | `(url: string, body?: string, headers?: object) → string` | Sends an HTTP POST request; returns the response body as a string. |

Pass headers as a plain object: `{ "Authorization": "Bearer token", "Content-Type": "application/json" }`.

```js
export function after_transform(items) {
    const config = JSON.parse(rb_read_file("/etc/my-tool/config.json"));
    const raw = rb_http_post(
        "https://api.example.com/enrich",
        JSON.stringify({ ids: items }),
        { "Authorization": "Bearer " + config.token, "Content-Type": "application/json" }
    );
    return JSON.parse(raw);
}
```

## Dependencies

| Crate | Role |
|-------|------|
| [`rquickjs`](https://github.com/DelSkayn/rquickjs) | Embedded QuickJS JavaScript runtime |
| [`clap`](https://github.com/clap-rs/clap) | CLI argument parsing |
| [`serde_json`](https://github.com/serde-rs/json) | JSON parsing and serialisation |
| [`anyhow`](https://github.com/dtolnay/anyhow) | Ergonomic error handling |
| [`reqwest`](https://github.com/seanmonstar/reqwest) | Blocking HTTP client for `rb_http_get` / `rb_http_post` |

## Project layout

```
src/main.rs            CLI entry-point
src/lib.rs             Pipeline logic and JS orchestration
examples/sample.json   Sample input
examples/sample.js     Sample transform script
docs/architecture.md   Design decisions and extension points
```

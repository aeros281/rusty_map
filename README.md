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
3. Calls the script's **default export** with the parsed JSON.
4. Pretty-prints the returned value to stdout.

## Script contract

The script must export a default function that accepts one argument and returns a JSON-serialisable value:

```js
export default function transform(data) {
  return { ...data, ok: true };
}
```

### Example

**`examples/sample.json`**
```json
{
  "users": [
    { "id": 1, "name": "Alice", "role": "admin" },
    { "id": 2, "name": "Bob",   "role": "viewer" }
  ],
  "version": "1.0"
}
```

**`examples/sample.js`**
```js
export default function transform(data) {
  const admins = data.users.filter((u) => u.role === "admin").map((u) => u.name);
  return {
    version: data.version,
    total_users: data.users.length,
    admins,
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
  "processed_at": "2026-04-26T00:00:00.000Z"
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

## Dependencies

| Crate | Role |
|-------|------|
| [`rquickjs`](https://github.com/DelSkayn/rquickjs) | Embedded QuickJS JavaScript runtime |
| [`clap`](https://github.com/clap-rs/clap) | CLI argument parsing |
| [`serde_json`](https://github.com/serde-rs/json) | JSON parsing and serialisation |
| [`anyhow`](https://github.com/dtolnay/anyhow) | Ergonomic error handling |

## Project layout

```
src/main.rs            CLI entry-point and runtime wiring
examples/sample.json   Sample input
examples/sample.js     Sample transform script
docs/architecture.md   Design decisions and extension points
```

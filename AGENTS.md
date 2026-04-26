# rusty_map

A Rust CLI that transforms JSON by executing a user-supplied JavaScript script.

## What it does

```
# Read JSON from a file via flag
rusty_map -f <json_file> <script_file>

# Pipe JSON via stdin
cat data.json | rusty_map <script_file>
```

1. Reads JSON from `-f <json_file>` or from stdin when the flag is omitted.
2. Loads `script_file` into an embedded QuickJS runtime ([rquickjs](https://github.com/DelSkayn/rquickjs)).
3. Calls the script's **default export** with the parsed JSON.
4. Pretty-prints the returned JSON to stdout.

## Script contract

The script must export a default function that accepts one argument and returns a JSON-serialisable value:

```js
export default function transform(data) {
  return { ...data, ok: true };
}
```

See [examples/sample.js](examples/sample.js) for a working example.

## Build & run

```sh
cargo build --release

# file flag
./target/release/rusty_map -f examples/sample.json examples/sample.js

# stdin pipe
cat examples/sample.json | ./target/release/rusty_map examples/sample.js
```

## Key files

| Path | Purpose |
|------|---------|
| `src/main.rs` | CLI entry-point and runtime wiring |
| `examples/sample.json` | Sample input |
| `examples/sample.js` | Sample transform script |
| `docs/architecture.md` | Design decisions and extension points |

## Updating this file

AGENTS.md is the **entry-point contract** for agents and developers.  
Keep it short: what the tool does, the script interface, and pointers to deeper docs.

Update it when:
- The CLI interface (flags / arguments) changes.
- The script contract changes (function signature, module format).
- A new key file is added or removed.

Do **not** put build troubleshooting, dependency rationale, or design notes here — those go in [docs/architecture.md](docs/architecture.md).

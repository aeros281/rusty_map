# Commit conventions

Commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>: <short summary>
```

The summary is lowercase, imperative mood, no trailing period, ≤ 72 characters.

## Types

| Type | When to use |
|------|-------------|
| `feat` | A new feature or user-visible capability |
| `fix` | A bug fix |
| `docs` | Documentation only — no code change |
| `refactor` | Code restructuring with no behaviour change |
| `test` | Adding or updating tests |
| `ci` | CI/CD pipeline changes (GitHub Actions, scripts) |
| `chore` | Tooling, dependencies, build config — nothing users care about |
| `style` | Formatting, whitespace — no logic change |

## Examples

```
feat: add generate-template subcommand
fix: handle empty stdin input gracefully
docs: document rb_http_post headers format
refactor: extract pipeline execution into helper
test: add filter+map integration test
ci: run cargo clippy in CI
chore: upgrade rquickjs to 0.12
```

## Breaking changes

Append `!` after the type and add a `BREAKING CHANGE:` footer:

```
feat!: require explicit subcommand for transform

BREAKING CHANGE: bare `rusty_map <script>` syntax removed; use `rusty_map transform <script>`
```

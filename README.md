# CSSForge

CSSForge is a safety-first semantic CSS refactoring and modernization workbench written in Rust.

This repository contains the first working implementation slice from the project requirements:

- reusable `cssforge-core` analysis/refactoring library;
- `cssforge` non-interactive CLI for analysis, planning, applying and rule discovery;
- Ratatui/Crossterm interactive TUI using the same core transformation plans;
- Lightning CSS as a strict semantic parse gate;
- lossless source-range scanning and patch-based rewriting for supported transforms;
- conservative adjacent/local native nesting transformations;
- transformation safety states and proof metadata;
- unified diff preview;
- dry-run, new-file, output-directory, overwrite-with-backup, overwrite, patch and stdout output modes;
- Git dirty-tree protection for destructive writes;
- JSON analysis/plan reports.

## Implemented safe refactors

The current build performs real transformations for immediately adjacent/local rules only:

- pseudo-class nesting: `.button:hover` → `.button { &:hover { ... } }`;
- pseudo-element nesting: `.card::before` → `.card { &::before { ... } }`;
- attribute-state nesting: `.button[disabled]`;
- compound-state nesting: `.item.active`, `.item#featured`;
- descendant nesting: `.card .title`;
- child/sibling combinator nesting: `.card > .title`, `.card + .peer`, `.card ~ .peer`;
- same-selector local `@media` nesting;
- same-selector local `@supports` nesting.

The refactorer refuses to move a candidate across comments or unrelated rules. Parent selector lists, BEM token-concatenation lookalikes and pseudo-element parent selectors are not grouped through `&` in this implementation slice.

Architectural invention of `@layer`, `@scope`, `@container`, and `@starting-style` is deliberately not an apply rule here. Existing occurrences are surfaced as semantic-review findings rather than silently rewritten.

## Requirements

The workspace is pinned to Rust 1.97.1 in `rust-toolchain.toml`.

## Build

```bash
cargo build --release
```

The binary will be:

```text
target/release/cssforge
```

## Run the TUI

```bash
cargo run -p cssforge -- interactive .
```

With no subcommand, interactive mode is the default:

```bash
cargo run -p cssforge --
```

For a specific file:

```bash
cargo run -p cssforge -- interactive ./styles.css
```

### TUI Step-by-Step Wizard Flow

CSSForge features an intuitive, 4-step workflow:
1. **Select Files**: Choose target CSS files (`Space` to toggle, `a` for all, `Enter` to confirm).
2. **Select Rules**: Choose modernization rules or presets (`Space` to toggle, `p` for preset, `Enter` to confirm).
3. **Output Settings & Review**: Select output mode (`↑/↓` or `1-7` to choose, `d` for diff preview, `Enter` to apply).
4. **Done**: View transformation summary and target paths (`Enter` or `q` to exit, `r` to start over).

| Key | Action |
|---|---|
| `Enter` | **Primary action**: Confirm & advance to next step / Apply & Finish |
| `Esc` / `Backspace` / `b` | Go back to previous step |
| `Space` | Toggle highlighted item (file / rule / output mode) |
| `↑` `↓` or `j` `k` | Move cursor / select option |
| `a` | Toggle all (files or rules) / refresh analysis |
| `p` | Cycle rule preset (Conservative ➔ Modern ➔ Aggressive ➔ Custom) |
| `d` / `v` | Inspect unified code diff and semantic proof checklist |
| `Tab` / `Shift+Tab` | Cycle steps forward / backward |
| `1`…`4` | Jump directly to step 1 (Files), 2 (Rules), 3 (Output), 4 (Done) |
| `r` | Rediscover CSS files from disk / Start over |
| `q` | Quit |

## CLI

Analyze:

```bash
cssforge analyze ./src --preset conservative
cssforge analyze ./src --preset modern --json
```

Generate plans without writing:

```bash
cssforge plan ./src --preset conservative
cssforge plan ./src --json
```

Apply to new sibling files (default non-destructive output):

```bash
cssforge apply ./src --preset conservative --output new-file
```

Dry run:

```bash
cssforge apply ./src --preset conservative --output dry-run
```

Write a mirrored tree to an output directory:

```bash
cssforge apply ./src --output out-dir --out-dir ./modern-css
```

Generate `.patch` files:

```bash
cssforge apply ./src --output patch
```

Overwrite only after explicit acknowledgement:

```bash
cssforge apply ./src --output overwrite-with-backup --yes
```

Destructive overwrite modes are refused when Git reports a dirty working tree.

List implemented rules:

```bash
cssforge rules
```

## Presets

- **Analysis** — no mutating rules enabled.
- **Conservative** — pseudo-class, pseudo-element, attribute, compound, local `@media`, and local `@supports` nesting.
- **Modern** — all currently implemented nesting/combinator rules.
- **Aggressive** — currently maps to the same concrete rule set as Modern; unsafe architectural transformations are intentionally not represented as fake toggles.
- **Custom** — TUI-managed manual rule selection.

## Safety model in this implementation

Every generated plan contains:

- safety classification;
- exact source range;
- original and proposed text;
- selector-set proof flag;
- specificity proof flag;
- cascade-context proof flag;
- source-order proof flag;
- layer/scope proof flags;
- declaration and `!important` preservation proof flags;
- human-readable reason and warnings.

Only selected `SAFE` plans are applied by the TUI. The CLI can opt into `REVIEW` plans with `--allow-review`, but this build currently generates mutation plans only for the conservative local-safe rules above.

## Source-preservation approach

Lightning CSS is used to reject input that cannot be parsed semantically. CSSForge does not reserialize the full stylesheet through Lightning CSS for these refactors. Instead, the source scanner records exact ranges and the writer replaces only the structural cluster being transformed. Declaration bodies are copied directly from the original source, preserving values, duplicate declarations and `!important` text inside those moved bodies.

Unchanged ranges are left untouched.

## Workspace

```text
cssforge/
├── Cargo.toml
├── rust-toolchain.toml
├── crates/
│   ├── cssforge-core/
│   │   └── src/
│   │       ├── discovery.rs
│   │       ├── engine.rs
│   │       ├── lib.rs
│   │       ├── model.rs
│   │       ├── output.rs
│   │       └── scanner.rs
│   ├── cssforge-tui/
│   │   └── src/
│   │       ├── app.rs
│   │       ├── lib.rs
│   │       └── ui.rs
│   └── cssforge-cli/
│       └── src/main.rs
└── LICENSE
```

## Test

```bash
cargo test --workspace
```

The core includes regression tests for pseudo/descendant nesting, local media nesting, BEM refusal, and selector-list refusal.

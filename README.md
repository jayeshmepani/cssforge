# CSSForge

CSSForge is a safety-first semantic CSS refactoring and modernization workbench written in Rust.

## Features

- Reusable `cssforge-core` analysis and refactoring engine library
- `cssforge` non-interactive CLI for analysis, planning, applying transforms, and rule discovery
- Ratatui/Crossterm interactive TUI with visual 4-step wizard workflow and unified diff preview
- Lightning CSS integration for strict semantic AST validation
- Lossless source-range scanning and patch-based rewriting
- Conservative adjacent and local native CSS nesting transformations
- Transformation safety states and proof verification metadata
- Flexible output modes: dry-run, new-file, output-directory, overwrite-with-backup, overwrite, patch, and stdout
- Git dirty-tree protection for destructive write operations
- Structured JSON reporting for integration with automated workflows

## Rules & Capabilities

CSSForge features **26 automated transformations** split into **Modernization** and **Structural Refactoring**:

### 1. Modernization (Native Nesting, Range Syntax & Selectors)

#### Native Nesting & Selector Factoring
- **Nest pseudo-classes** (`nest-pseudo-class`): `.button:hover` → `.button { &:hover { ... } }`
- **Nest pseudo-elements** (`nest-pseudo-element`): `.card::before` → `.card { &::before { ... } }`
- **Nest attribute states** (`nest-attribute`): `.button[disabled]` → `.button { &[disabled] { ... } }`
- **Nest compound states** (`nest-compound`): `.item.active` → `.item { &.active { ... } }`
- **Nest descendants** (`nest-descendant`): `.card .title` → `.card { .title { ... } }`
- **Nest combinators** (`nest-combinator`): `.card > .title`, `.card + .peer`, `.card ~ .peer`
- **Factor selector lists** (`factor-selector-list`): Factor comma-separated selectors sharing a base (`.marker, .marker::before` → `.marker { &, &::before }`)

#### At-Rule Inlining
- **Nest local `@media`** (`nest-media`): Inline immediately-following `@media` blocks with matching selectors into nested rules
- **Nest local `@supports`** (`nest-supports`): Inline immediately-following `@supports` blocks with matching selectors into nested rules
- **Nest local `@container`** (`nest-container`): Inline immediately-following `@container` query blocks into nested rules
- **Nest `@starting-style`** (`nest-starting-style`): Inline immediately-following `@starting-style` blocks into parent selector rules

#### Modern Selectors & Media Syntax
- **Consolidate `:not()` selectors** (`consolidate-not`): Consolidate chained `:not()` selectors (`:not(a):not(b)` → `:not(a, b)`)
- **Factor with `:is()`** (`modernize-is`): Factor selector-list alternatives with uniform specificity into `:is(...)` grouping
- **Modernize with `:where()`** (`modernize-where`): Factor selector-list alternatives into `:where(...)` for zero-specificity defaults
- **Modernize media range syntax** (`modernize-media-range-syntax`): Convert `min-width` / `max-width` / `min-height` / `max-height` to CSS Range Syntax (e.g. `(width >= 800px)`)

---

### 2. Refactoring (Consolidation, Deduplication & Structural Cleanup)

#### At-Rule Block Merging
- **Merge same named `@layer` blocks** (`merge-same-named-layer`): Consolidate separated blocks of the same named `@layer` into their canonical first occurrence
- **Merge adjacent `@media` queries** (`merge-adjacent-media`): Combine consecutive `@media` blocks having identical query conditions
- **Merge adjacent `@supports` queries** (`merge-adjacent-supports`): Combine consecutive `@supports` blocks having identical feature conditions
- **Merge adjacent `@container` queries** (`merge-adjacent-container`): Combine consecutive `@container` blocks having identical container name & query conditions
- **Merge adjacent `@scope` blocks** (`merge-identical-scope`): Combine consecutive `@scope` blocks having identical root and limit parameters
- **Merge adjacent `@starting-style` blocks** (`merge-identical-starting-style`): Combine consecutive top-level `@starting-style` blocks into a single block

#### Selector & Body Deduplication
- **Merge adjacent identical selectors** (`merge-adjacent-identical-selector`): Combine consecutive style rules sharing exact same selector
- **Merge identical rule bodies** (`merge-identical-rule-bodies`): Combine selectors sharing identical declaration bodies into a unified comma-separated rule
- **Factor identical states with `:is()`** (`factor-identical-states-with-is`): Combine multiple states of the same element sharing identical bodies into `&:is(:hover, :focus, ...)` form
- **Gather related selector rules** (`gather-related-selector-rules`): Gather scattered non-adjacent occurrences of the same selector — including exact duplicates, directly-attached pseudo-classes/elements (`&:hover`, `&::before`), combinator variants (`+ *`, `> .child`, `~ .peer`), and whitespace-descendant variants (`:not(*)`) — into the canonical first rule block
- **Prune overridden declarations & rules** (`prune-overridden-declarations`): Remove dead declarations and entire rules overridden by later identical selectors in the cascade

Safety rules strictly prohibit moving candidate rules across comments or unrelated CSS blocks. Parent selector lists, BEM token-concatenation lookalikes, and pseudo-element parent selectors are safely preserved.

## Requirements

- Rust 1.85+ (Edition 2024, pinned to toolchain `1.97.1` in `rust-toolchain.toml`)

## Build

```bash
cargo build --release
```

The binary will be created at:

```text
target/release/cssforge
```

## Interactive TUI

Run interactive mode in the current directory:

```bash
cargo run -p cssforge --
```

Or target a specific path:

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

## CLI Usage

Analyze:

```bash
cssforge analyze ./src --preset conservative
cssforge analyze ./src --preset modern --json
```

Generate transformation plans without modifying files:

```bash
cssforge plan ./src --preset conservative
cssforge plan ./src --json
```

Apply to new sibling files (default non-destructive mode):

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

Overwrite with backup (requires acknowledgement):

```bash
cssforge apply ./src --output overwrite-with-backup --yes
```

*Destructive overwrite modes are automatically blocked if Git reports an uncommitted working tree.*

List available modernization rules:

```bash
cssforge rules
```

## Presets

- **Analysis** — No mutating rules enabled; surfaces modernization findings only.
- **Conservative** — Enables pseudo-class, pseudo-element, attribute, compound, local `@media`, and local `@supports` nesting.
- **Modern** — Enables all implemented nesting and combinator rules.
- **Aggressive** — Maps to the modern rule set; unsafe architectural transformations are intentionally excluded.
- **Custom** — Manual rule selection via the interactive TUI.

## Safety Model

Every generated transformation plan includes verification proof flags:

- Safety classification (`SAFE`, `REVIEW`, `UNSAFE`)
- Exact source byte ranges
- Original and proposed CSS text
- Selector-set proof flag
- Specificity proof flag
- Cascade-context proof flag
- Source-order proof flag
- Layer/scope proof flags
- Declaration and `!important` preservation proof flags
- Human-readable reason and warnings

Only verified `SAFE` plans are applied by default. The CLI can opt into `REVIEW` plans with `--allow-review`.

## Source-Preservation Approach

Lightning CSS is used to validate CSS syntax and AST structures. CSSForge does not reserialize the full stylesheet through Lightning CSS; instead, the source scanner targets exact byte ranges and replaces only the structural cluster being transformed. Declaration bodies are preserved directly from the original source text, retaining formatting, comments, custom properties, and `!important` flags.

Unchanged ranges remain untouched.

## Workspace Structure

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
│   │       ├── banner.rs
│   │       ├── lib.rs
│   │       └── ui.rs
│   └── cssforge-cli/
│       └── src/main.rs
└── LICENSE
```

## Testing

Run workspace test suite:

```bash
cargo test --workspace
```


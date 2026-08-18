# CSSForge

[![Crates.io Version](https://img.shields.io/crates/v/cssforge.svg?style=flat-square)](https://crates.io/crates/cssforge)
[![Total Downloads](https://img.shields.io/crates/d/cssforge.svg?style=flat-square)](https://crates.io/crates/cssforge)
[![Documentation](https://img.shields.io/docsrs/cssforge?style=flat-square)](https://docs.rs/cssforge)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-blue.svg?style=flat-square)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg?style=flat-square)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Status](https://img.shields.io/badge/status-stable-brightgreen.svg?style=flat-square)](https://github.com/jayeshmepani/cssforge/releases)

**CSSForge** is a safety-first, lossless semantic CSS refactoring and modernization workbench written in Rust. It transforms legacy CSS stylesheets into modern native CSS features (native nesting, Range media queries, `:is()` factoring, `@layer` consolidation, and dead-code pruning) while guaranteeing **zero declaration loss** and surgical byte-level preservation.

---

## 📦 Published Workspace Packages

CSSForge is architected as a modular workspace published across three official crates on [crates.io](https://crates.io):

| Package | Type | Crates.io | Description |
| :--- | :--- | :---: | :--- |
| [**`cssforge`**](https://crates.io/crates/cssforge) | **CLI / TUI Binary** | [![crates.io](https://img.shields.io/crates/v/cssforge.svg)](https://crates.io/crates/cssforge) | **The Main Product**: Standalone executable with both an interactive visual terminal UI (TUI workbench) and a fast headless CLI. |
| [**`cssforge-core`**](https://crates.io/crates/cssforge-core) | **Library (Pure Rust)** | [![crates.io](https://img.shields.io/crates/v/cssforge-core.svg)](https://crates.io/crates/cssforge-core) | **The Core Engine**: Headless AST parser, specificity calculator, selector hierarchy tree, and 26 transformation rules with zero UI dependencies. |
| [**`cssforge-tui`**](https://crates.io/crates/cssforge-tui) | **UI Component** | [![crates.io](https://img.shields.io/crates/v/cssforge-tui.svg)](https://crates.io/crates/cssforge-tui) | **The Terminal UI**: Reusable Ratatui/Crossterm interface, ASCII banner, step-by-step wizard, and unified diff viewer. |

---

## 🚀 Installation & Quick Start

### Option 1: Install via Cargo (Recommended)

Install the global CLI & TUI tool directly from [crates.io](https://crates.io/crates/cssforge):

```bash
cargo install cssforge
```

### Option 2: Pre-compiled Binaries (GitHub Releases)

Download pre-built, native standalone binaries from [GitHub Releases](https://github.com/jayeshmepani/cssforge/releases) for your OS & CPU architecture:

* **Windows (x64 Intel/AMD)**: `cssforge-v0.2.0-windows-x64.zip`
* **Windows (ARM64 / Snapdragon)**: `cssforge-v0.2.0-windows-arm64.zip`
* **Linux (x64 Intel/AMD)**: `cssforge-v0.2.0-linux-x64.tar.gz`
* **Linux (ARM64 / Snapdragon)**: `cssforge-v0.2.0-linux-arm64.tar.gz`
* **macOS (Apple Silicon M-Series)**: `cssforge-v0.2.0-macos-arm64.tar.gz`

### Option 3: Use as a Rust Library Dependency

Add the core engine to your Rust project's `Cargo.toml`:

```toml
[dependencies]
cssforge-core = "0.2.0"
```
Or via CLI:
```bash
cargo add cssforge-core
```

---

## 🖥️ Two Operating Modes

### 1. Interactive Terminal UI (TUI Workbench)

Simply run `cssforge` without arguments in your project directory:

```bash
cssforge
```

Or target a specific directory:
```bash
cssforge interactive ./styles
```

#### 4-Step Visual Wizard Workflow
1. **Select Files**: Choose target stylesheets (`Space` to toggle, `a` for all, `Enter` to confirm).
2. **Select Rules & Presets**: Toggle individual rules or press `p` to cycle presets (`Conservative` ➔ `Modern` ➔ `Refactor` ➔ `Aggressive`).
3. **Output Mode & Diff Inspection**: Select output strategy (`New file`, `Overwrite`, `Patch`, etc.) and press `d` to preview the side-by-side unified diff.
4. **Done**: View transformation summary and target paths.

| Key | Action |
|---|---|
| `Enter` | **Primary action**: Advance to next step / Apply & Finish |
| `Esc` / `Backspace` / `b` | Go back to previous step |
| `Space` | Toggle highlighted item (file / rule / output mode) |
| `↑` `↓` or `j` `k` | Move cursor |
| `a` | Toggle all files / rules |
| `p` | Cycle preset (`Conservative` ➔ `Modern` ➔ `Refactor` ➔ `Aggressive` ➔ `Custom`) |
| `d` / `v` | Inspect live unified code diff & safety proof checklist |
| `Tab` / `Shift+Tab` | Navigate between wizard steps |
| `1`…`4` | Jump directly to step 1 (Files), 2 (Rules), 3 (Output), 4 (Done) |
| `r` | Refresh files from disk / Start over |
| `q` | Quit |

---

### 2. Fast Non-Interactive CLI (Headless Mode)

Perfect for CI/CD pipelines, npm scripts, and Git pre-commit hooks:

```bash
# Analyze CSS files and report modernization findings
cssforge analyze ./src --preset modern

# Analyze and output structured JSON report
cssforge analyze ./src --json

# Generate transformation plans without writing files
cssforge plan ./src --preset modern

# Apply transformations to new sibling files (*.modern.css)
cssforge apply ./src --preset modern --output new-file

# Apply dry-run
cssforge apply ./src --output dry-run

# Output modernized code to a dedicated directory
cssforge apply ./src --output out-dir --out-dir ./dist/css

# Generate unified .patch files
cssforge apply ./src --output patch

# Overwrite in place (with automatic backup .bak file)
cssforge apply ./src --output overwrite-with-backup --yes

# List all implemented rules
cssforge rules
```

---

## 🛠️ Complete Rules & Capabilities (26 Rules)

CSSForge provides **26 automated transformations** categorized into **Modernization** and **Structural Refactoring**:

### Section 1: Modernize (Native Nesting, Range Syntax & Selectors)

#### Native Nesting & Selector Factoring
* **Nest pseudo-classes** (`nest-pseudo-class`): `.button:hover` → `.button { &:hover { ... } }`
* **Nest pseudo-elements** (`nest-pseudo-element`): `.card::before` → `.card { &::before { ... } }`
* **Nest attribute states** (`nest-attribute`): `.button[disabled]` → `.button { &[disabled] { ... } }`
* **Nest compound states** (`nest-compound`): `.item.active` → `.item { &.active { ... } }`
* **Nest descendants** (`nest-descendant`): `.card .title` → `.card { .title { ... } }`
* **Nest combinators** (`nest-combinator`): `.card > .title`, `.card + .peer`, `.card ~ .peer`
* **Factor selector lists** (`factor-selector-list`): Factor comma-separated selectors sharing a base (`.marker, .marker::before` → `.marker { &, &::before }`)

#### Conditional At-Rule Nesting
* **Nest local `@media`** (`nest-media`): Inline immediately-following `@media` blocks with matching selectors into nested rules
* **Nest local `@supports`** (`nest-supports`): Inline immediately-following `@supports` blocks with matching selectors into nested rules
* **Nest local `@container`** (`nest-container`): Inline immediately-following `@container` query blocks into nested rules
* **Nest `@starting-style`** (`nest-starting-style`): Inline immediately-following `@starting-style` blocks into parent selector rules

#### Modern Selectors & Range Syntax
* **Consolidate `:not()` selectors** (`consolidate-not`): Consolidate chained `:not()` selectors (`:not(a):not(b)` → `:not(a, b)`)
* **Factor with `:is()`** (`modernize-is`): Factor selector-list alternatives with uniform specificity into `:is(...)` grouping
* **Modernize with `:where()`** (`modernize-where`): Factor selector-list alternatives into `:where(...)` for zero-specificity defaults
* **Modernize media range syntax** (`modernize-media-range-syntax`): Convert legacy min/max queries to CSS Range Syntax (e.g. `(width >= 800px)`)

---

### Section 2: Refactor (Consolidation, Deduplication & Structural Cleanup)

#### At-Rule Block Merging
* **Merge same named `@layer` blocks** (`merge-same-named-layer`): Consolidate separated blocks of the same named `@layer` into their canonical first occurrence
* **Merge adjacent `@media` queries** (`merge-adjacent-media`): Combine consecutive `@media` blocks having identical query conditions
* **Merge adjacent `@supports` queries** (`merge-adjacent-supports`): Combine consecutive `@supports` blocks having identical feature conditions
* **Merge adjacent `@container` queries** (`merge-adjacent-container`): Combine consecutive `@container` blocks having identical container name & query conditions
* **Merge adjacent `@scope` blocks** (`merge-identical-scope`): Combine consecutive `@scope` blocks having identical root and limit parameters
* **Merge adjacent `@starting-style` blocks** (`merge-identical-starting-style`): Combine consecutive top-level `@starting-style` blocks into a single block

#### Selector & Body Deduplication
* **Merge adjacent identical selectors** (`merge-adjacent-identical-selector`): Combine consecutive style rules sharing exact same selector
* **Merge identical rule bodies** (`merge-identical-rule-bodies`): Combine selectors sharing identical declaration bodies into a unified comma-separated rule
* **Factor identical states with `:is()`** (`factor-identical-states-with-is`): Combine multiple states of the same element sharing identical bodies into `&:is(:hover, :focus, ...)` form
* **Gather related selector rules** (`gather-related-selector-rules`): Gather scattered non-adjacent occurrences of the same selector into the canonical first rule block
* **Prune overridden declarations & rules** (`prune-overridden-declarations`): Remove dead declarations and entire rules overridden by later identical selectors in the cascade

---

## 🔒 Safety & Source Preservation Guarantees

1. **Zero Declaration Loss**: CSSForge never deletes declarations or drops valid styles.
2. **Surgical Byte-Range Preservation**: Lightning CSS validates the AST, but CSSForge splices replacements strictly within target byte spans. Untouched code remains 100% byte-for-byte identical.
3. **Proof Verification**: Every generated transformation plan is validated against 8 proof invariants (specificity, cascade order, layer equivalence, and declaration exactness).
4. **Git Dirty-Tree Guard**: In-place destructive overwrite modes are blocked if Git reports uncommitted working changes.
5. **Strictly Forward Engine**: Reverse/backward demodernization (de-nesting) is intentionally unsupported to guarantee strict forward modernization integrity.

---

## 🏗️ Repository & Workspace Architecture

```text
cssforge/
├── Cargo.toml                  # Workspace manifest
├── rust-toolchain.toml         # Pinned Rust toolchain (1.97.1)
├── crates/
│   ├── cssforge-core/          # Headless core analysis & transformation engine
│   │   └── src/
│   │       ├── discovery.rs    # Safe file system discovery
│   │       ├── engine.rs       # 26 AST refactoring rules & plan generator
│   │       ├── model.rs        # Data structures, proofs, and presets
│   │       ├── output.rs       # Atomic file writers & patch generators
│   │       └── scanner.rs      # Source-range scanner & AST tokenizer
│   ├── cssforge-tui/           # Terminal UI workbench
│   │   └── src/
│   │       ├── app.rs          # State management & keyboard event handlers
│   │       ├── banner.rs       # Responsive ASCII logo banner
│   │       └── ui.rs           # Ratatui view renderers & diff viewer
│   └── cssforge-cli/           # Main binary entrypoint (CLI & TUI launcher)
│       └── src/main.rs
├── examples/
│   ├── demo.css                # Reference CSS test fixture
│   └── demo.modern.css         # Modernized output artifact
└── LICENSE                     # MIT License
```

---

## 🧪 Testing

Run the full workspace test suite:

```bash
cargo test --workspace --locked
```

Run linter checks:

```bash
cargo clippy --all-targets -- -D warnings
```

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

Developed with ❤️ by **Jayesh Mepani** ([@jayeshmepani](https://github.com/jayeshmepani)).

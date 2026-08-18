# CSSForge

[![Crates.io Version](https://img.shields.io/crates/v/cssforge.svg?style=flat-square)](https://crates.io/crates/cssforge)
[![Total Downloads](https://img.shields.io/crates/d/cssforge.svg?style=flat-square)](https://crates.io/crates/cssforge)
[![Documentation](https://img.shields.io/docsrs/cssforge?style=flat-square)](https://docs.rs/cssforge)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-blue.svg?style=flat-square)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg?style=flat-square)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Status](https://img.shields.io/badge/status-stable-brightgreen.svg?style=flat-square)](https://github.com/jayeshmepani/cssforge/releases)

**CSSForge** is a safety-first, lossless semantic CSS refactoring engine and interactive terminal workbench written in Rust. It modernizes flat legacy CSS into native nesting, Range media queries, `:is()` factoring, and `@layer` consolidation with **zero declaration loss** and byte-range surgical precision.

📖 **Full Interactive Documentation & Rules Catalog**: [https://jayeshmepani.github.io/cssforge/](https://jayeshmepani.github.io/cssforge/) *(or open `docs/index.html` locally)*.

---

## 📦 Published Workspace Crates

| Package | Type | Crates.io | Description |
| :--- | :--- | :---: | :--- |
| [**`cssforge`**](https://crates.io/crates/cssforge) | **CLI / TUI Binary** | [![crates.io](https://img.shields.io/crates/v/cssforge.svg?style=flat-square)](https://crates.io/crates/cssforge) | Standalone executable with both an interactive visual terminal UI (TUI workbench) and headless CLI. |
| [**`cssforge-core`**](https://crates.io/crates/cssforge-core) | **Pure Rust Library** | [![crates.io](https://img.shields.io/crates/v/cssforge-core.svg?style=flat-square)](https://crates.io/crates/cssforge-core) | Headless AST parser, specificity calculator, and 26 transformation rules with zero UI dependencies. |
| [**`cssforge-tui`**](https://crates.io/crates/cssforge-tui) | **UI Component** | [![crates.io](https://img.shields.io/crates/v/cssforge-tui.svg?style=flat-square)](https://crates.io/crates/cssforge-tui) | Reusable Ratatui/Crossterm interface, ASCII banner, step-by-step wizard, and unified diff viewer. |

---

## 🚀 Installation

### Option 1: Via Cargo (All Platforms — Recommended)

```bash
cargo install cssforge
```

### Option 2: Pre-compiled Standalone Binaries

Download from [GitHub Releases](https://github.com/jayeshmepani/cssforge/releases):

#### 🐧 Linux (x64 / ARM64)
```bash
# Extract and copy to local user bin (no sudo needed):
tar -xzf cssforge-v0.2.0-linux-x64.tar.gz
cp cssforge-v0.2.0-linux-x64/cssforge ~/.local/bin/
chmod +x ~/.local/bin/cssforge
```

#### 🍏 macOS (Apple Silicon M-Series)
```bash
tar -xzf cssforge-v0.2.0-macos-arm64.tar.gz
sudo cp cssforge-v0.2.0-macos-arm64/cssforge /usr/local/bin/
chmod +x /usr/local/bin/cssforge
```

#### 🪟 Windows (x64 / ARM64 Snapdragon)
Extract `cssforge.exe` from `cssforge-v0.2.0-windows-x64.zip` and move it to any directory in your system `Path` (e.g. `C:\Windows\System32` or your tools folder).

---

## 💡 Usage in Any Project

CSSForge automatically scans for CSS files in the **current working directory** or any path you pass:

### 1. Interactive TUI Workbench
```bash
# Go to ANY web project folder:
cd /path/to/my-project

# Launch interactive modernization:
cssforge

# Or target a specific folder / file:
cssforge interactive ./src/css
```

#### Keyboard Shortcuts
* `[Enter]` Next step / Apply
* `[Space]` Toggle file or rule selection
* `[a]` Select / Deselect All
* `[p]` Cycle presets (`Conservative` ➔ `Modern` ➔ `Refactor` ➔ `Aggressive`)
* `[d]` / `[v]` Open live unified code diff & safety proof checklist
* `[q]` Quit

---

### 2. Headless CLI (CI/CD & Automation)
```bash
# Analyze CSS files and report modernization findings
cssforge analyze ./src

# Analyze with structured JSON output
cssforge analyze ./src --json

# Apply modern preset to new files (*.modern.css)
cssforge apply ./src/app.css --preset modern --output new-file

# Overwrite in-place with automatic safety backup (.bak)
cssforge apply ./src/app.css --output overwrite-with-backup --yes

# List all 26 transformation rules
cssforge rules
```

---

## 🛠️ 26 Transformation Rules Summary

* **Native Nesting**: `nest-pseudo-class`, `nest-pseudo-element`, `nest-attribute`, `nest-compound`, `nest-descendant`, `nest-combinator`, `factor-selector-list`.
* **Conditional At-Rules**: `nest-media`, `nest-supports`, `nest-container`, `nest-starting-style`.
* **Modern Selectors**: `consolidate-not`, `modernize-is`, `modernize-where`, `modernize-media-range-syntax`.
* **At-Rule Merging**: `merge-same-named-layer`, `merge-adjacent-media`, `merge-adjacent-supports`, `merge-adjacent-container`, `merge-identical-scope`, `merge-identical-starting-style`.
* **Deduplication & Pruning**: `merge-adjacent-identical-selector`, `merge-identical-rule-bodies`, `factor-identical-states-with-is`, `gather-related-selector-rules`, `prune-overridden-declarations`.

For complete interactive visual examples of each rule, visit the [Documentation Site](https://jayeshmepani.github.io/cssforge/).

---

## 🔒 Safety Guarantees & Why CSSForge

Unlike lowering tools (e.g. LightningCSS, esbuild) or destructive minifiers (e.g. cssnano), CSSForge is built strictly for **lossless forward semantic modernization**:

* **🛡️ Dual-Layer Zero-Regression Engine**: LightningCSS validates the AST, but a surgical Byte Patch Engine mutates only the targeted byte ranges. Untouched lines, developer comments, custom indentation, and quote styles remain 100% byte-for-byte identical.
* **🔬 Mathematical Proof Engine**: Calculates exact specificity vectors `(a, b, c)` with zero specificity drift guarantees.
* **🚫 Refusal as a Safety Feature**: Refuses transformations that would break CSS matching (such as `:is()` specificity inflation on lower branches).
* **✨ Multi-Selector Cluster Factoring**: Automatically factors multi-branch rules sharing identical bases into clean `:is()` blocks with nested children.
* **🔒 Git Dirty-Tree Guard**: Blocks destructive in-place replacements if uncommitted Git changes are detected.

---

## 🚫 Strict Non-Goals (What We Do NOT & Will NOT Do)

* ❌ **No BEM String Concatenation (`&__element`)**: Native CSS `&` is a selector token (desugars to `:is()`), NOT a Sass string concatenator. Writing `.card { &__title { } }` is invalid in native CSS.
* ❌ **No Specificity Inflation / Lifting**: If wrapping parent selectors in `:is()` would artificially lift the specificity of a lower-specificity branch and alter cascade priority, CSSForge refuses the refactor.
* ❌ **No Destructive Re-Serialization**: We never re-format untouched code, strip comments, convert colors, or drop intentional browser fallback duplicate declarations.
* ❌ **No Unsound At-Rule Moving Across Barriers**: We never hoist `@media` / `@supports` blocks across intervening selector barriers if moving them would invert the cascade.

---

## 📄 License

MIT © [Jayesh Mepani](https://github.com/jayeshmepani)

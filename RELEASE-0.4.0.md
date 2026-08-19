# CSSForge 0.4.0

Minor release on the 0.x line: one new transformation rule and gather behavior that no longer moves declarations across cascade layers.

## Why 0.4.0 (not 0.3.2)

- New public rule: `nest-layer-by-selector` (27 rules total).
- Gather no longer dumps a selector from `@layer base` into `@layer tokens`.
- Existing gather plans are atomic (no orphan deletes that drop chrome).

## New rule: `nest-layer-by-selector`

Same exact selector in multiple **named** `@layer` blocks is hoisted out of those layers and rewritten as nested `@layer` blocks, which keep layer identity (not a child layer such as `tokens.base`):

```css
@layer tokens { :root { A } }
@layer base { :root { B } }

/* becomes */

:root {
  @layer tokens { A }
  @layer base { B }
}
```

Layer order from an existing `@layer reset, tokens, base, …` list is unchanged. The rule does **not** hoist when an unlayered rule sits between the layers.

## Gather is layer-scoped

`gather-related-selector-rules` only merges related rules **inside the same named layer** (or both unlayered). It will not copy `@layer base` declarations into `@layer tokens`.

At-rule preludes (`@layer tokens`, `@media …`) are not factored as selector prefixes, so a second apply cannot smash named layers into `@layer { tokens {} base {} }`.

## Other gather fixes in this line

- Mixed `@supports` with a dominant home (e.g. `.custom-select` plus `selectedcontent`) inverts into the home instead of deleting chrome.
- Gather emits one spanning plan so overlapping nest plans cannot orphan delete-only plans.
- Related nests factor deeper: `button { &:hover }`, `&:open { … }`, `&.compact { … }`.
- Prefix-only sibling nesting (no `&:open &` / `&:focus &` rewrite of descendants).

## Docs

README and the HTML rule catalog list all **27** rules, including `nest-layer-by-selector`.

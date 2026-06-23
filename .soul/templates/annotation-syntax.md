# Markdown HTML-comment annotation syntax

Use this to attach Soul annotations to any `.md` file that does **not** have YAML frontmatter — READMEs, design docs, ADRs, etc.

## Format

```html
<!-- soul id="dot.separated.id" key="value" key2="value2" -->
```

## Rules

- `id` is required; all other keys are optional metadata
- Values must be double-quoted: `key="value"`
- Space-separated key-value pairs, HTML-attribute style
- Must occupy the entire trimmed line (no surrounding text)
- Single-line only
- Backslash escapes supported: `\"`, `\\`, `\n`, `\r`, `\t`

## Examples

```markdown
<!-- soul id="arch.adr-001" layer="docs" -->
<!-- soul id="indexer.scan-repository" layer="reference" -->
```

# SemanticGraph Codex Assets

This directory is the generated Codex asset snapshot for SemanticGraph Phase 3. It is checked in so future installer work can compare, copy, or structurally merge known-good artifacts without re-deriving their content.

Generated assets include:

- `.agents/skills/semantic-graph/SKILL.md`
- `.agents/skills/semantic-graph/references/*.md`
- `.codex/agents/semantic-graph-*.toml`
- `.codex/config.semantic-graph.toml`

The config file is a snippet for later project-local merge into `.codex/config.toml`. It is not an install manifest and does not claim ownership of user files.

Regenerate the snapshot with `semantic-graph-agent-assets generate`. Check for drift with `semantic-graph-agent-assets check`.

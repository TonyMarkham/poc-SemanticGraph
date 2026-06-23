# Soul Doc Template

Copy this file when creating a new Soul document. Delete all placeholder text (lines in `[brackets]`).
The frontmatter `id`, `kind`, and `title` are required — Soul will not index the doc without them.

---

```markdown
---
id: [dot.separated.semantic.id]
kind: [concept | interaction | policy | decision]
title: [Short human-readable title]
---

## Overview

[1–3 sentences. What is this? Where does it fit? No implementation detail yet.]

## Five W's

**Who** — [Which role or system actor initiates or owns this? Be specific: "platform admin", "orchestrator process", "coach", "student". Not "the system" or "the backend".]

**What** — [What does this actually do? One focused sentence on the operation or concept, not the implementation.]

**When** — [Lifecycle position. When does this happen relative to other events? What preconditions must be true? What triggers it?]

**Where** — Covered by Soul annotations (see linked code locations).

**Why** — [The design rationale. Why was this built this way? What problem does it solve? What alternatives were rejected and why? This is the hardest W to write and the most valuable one.]

### Needs elaboration

> Use this section to flag anything that could not be answered from the code alone.
> Each bullet should be a specific question, not a vague note.
> Mark answered items with ✓ and the date so the history is visible.

- [ ] [Open question requiring design input]
- [ ] [Open question requiring business context]
- [ ] [Potential risk or bug worth tracking]

---

## [Technical section — e.g. Steps, Behaviour, Schema]

[Implementation detail goes here, below the Five W's. This section is for the "how", not the "why".]

## [Additional sections as needed]
```

---

## Kind reference

| Kind | Use when |
|---|---|
| `concept` | A design pattern, abstraction, or system component (e.g. the vault provider, the process model) |
| `interaction` | A discrete user- or admin-triggered operation with a clear before/after (e.g. provision tenant, delete key) |
| `policy` | A rule or constraint the system enforces (e.g. tenant key naming convention, rate limiting) |
| `decision` | An architectural decision with context and trade-offs (pairs well with ADR format in the body) |

## ID convention

IDs are dot-separated and hierarchical. Follow the existing namespace patterns:

```
vault.provider
vault.secret.get
vault.tenant.provision
auth.clerk.webhook
stripe.connect.onboarding
```

Use lowercase kebab-case within each segment. IDs are permanent once annotations reference them — rename carefully.

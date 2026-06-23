# Prompt — Audit Soul Docs

Use this prompt when you want Claude to run the audit rubric against an existing set of Soul docs and produce a scored report.

---

## Prompt

```
Using the audit rubric at .soul/templates/audit-rubric.md, audit the Soul docs for [namespace or list of Soul IDs].

For each doc:
1. Score it against the rubric (✓ / ~ / ✗ / ? per item)
2. Give an overall percentage score and status label
3. List every gap as a specific, actionable finding — not vague observations

After scoring all docs, summarise:
- Which gaps you can fill immediately from the codebase (Code)
- Which gaps require my input (Human)
- Which gaps are potential bugs or risks (Risk)

Do not fill the gaps yet — just identify and categorise them. I will tell you which ones to address.
```

---

## Example

```
Using the audit rubric at .soul/templates/audit-rubric.md, audit the Soul docs for auth.*.

For each doc:
1. Score it against the rubric (✓ / ~ / ✗ / ? per item)
2. Give an overall percentage score and status label
3. List every gap as a specific, actionable finding — not vague observations

After scoring all docs, summarise:
- Which gaps you can fill immediately from the codebase (Code)
- Which gaps require my input (Human)
- Which gaps are potential bugs or risks (Risk)

Do not fill the gaps yet — just identify and categorise them. I will tell you which ones to address.
```

---

## Variants

**Audit a single doc:**
```
Audit the Soul doc for [soul.id] using .soul/templates/audit-rubric.md. Score it, list all gaps, and categorise each as Code / Human / Risk.
```

**Audit and fill in one pass:**
```
Audit the Soul docs for [namespace] using .soul/templates/audit-rubric.md. Fill any gaps you can derive from the codebase immediately. Flag the rest as Human or Risk in the Needs elaboration section of each doc.
```

**Audit before starting a feature:**
```
Before we start work on [feature], audit the Soul doc for [soul.id] to confirm it is complete and accurate. Flag any gaps that could affect implementation decisions.
```

---

## Tips

- Namespace wildcards work well here: `vault.*`, `auth.*`, `stripe.*`
- Ask for audit-only (no fills) when you want to review the gaps before deciding what to address
- The Risk category is worth scanning first — those may be bugs, not just doc gaps

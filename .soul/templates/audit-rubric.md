# Soul Doc Audit Rubric

Run this checklist against any Soul doc or set of docs. For each item, mark:
- ✓ — present and satisfactory
- ~ — present but thin / needs improvement
- ✗ — absent
- ? — cannot determine from code alone (needs human input)

---

## Structure

- [ ] Frontmatter has `id`, `kind`, and `title`
- [ ] `id` follows the dot-separated namespace convention and matches the annotation in code
- [ ] `kind` is appropriate for what the doc describes (`concept` / `interaction` / `policy` / `decision`)
- [ ] Overview section exists and is ≤3 sentences
- [ ] Five W's section exists with all five addressed
- [ ] "Needs elaboration" section exists (even if empty — empty means the doc is complete)
- [ ] Technical detail is below the Five W's, not mixed in with them

---

## Five W's quality

### Who
- [ ] Names a specific role or actor — not "the system", "the backend", or "the user"
- [ ] Distinguishes between who *triggers* the operation and who *benefits* from it (if different)
- [ ] Notes if the actor is human or automated

### What
- [ ] Describes the operation at the right level of abstraction — not too high ("does stuff"), not too low (implementation detail)
- [ ] Is distinct from the Overview (Overview = context, What = the operation itself)

### When
- [ ] States lifecycle position (startup / request time / scheduled / on-demand / event-driven)
- [ ] States preconditions that must be true
- [ ] States what event or action triggers it
- [ ] Notes frequency (once / per-request / per-tenant / on-demand)

### Where
- [ ] Defers to Soul annotations (should not duplicate file paths — that's the indexer's job)
- [ ] If there are layers not yet annotated, flags them in Needs Elaboration

### Why
- [ ] Explains a *decision*, not just a description of the code
- [ ] Addresses at least one alternative that was rejected or a constraint that shaped the design
- [ ] Is not just a restatement of What ("Why: because we need to store the key" is not a why)

---

## Coverage checks

- [ ] Partial failure / rollback behaviour addressed (or explicitly noted as unhandled)
- [ ] Error conditions and their consequences described
- [ ] Cross-cutting concerns addressed where relevant:
  - [ ] Authentication / authorisation (who is allowed to trigger this?)
  - [ ] Audit trail (is this operation logged or persisted for compliance?)
  - [ ] Idempotency (what happens if this is called twice?)
  - [ ] Concurrency (what happens if two callers race?)
- [ ] Relationship to adjacent Soul IDs noted (what calls this, what does this call)

---

## Needs elaboration quality

- [ ] Every open question is specific and actionable — not vague
- [ ] Questions are categorised: design input needed / business context needed / potential bug
- [ ] Previously answered questions are marked ✓ with a date
- [ ] Section is not used to dump implementation notes that belong in the technical sections

---

## Scoring guide

Count ✓ marks out of total applicable items.

| Score | Status |
|---|---|
| 90–100% | Complete — minor polish only |
| 70–89% | Good — targeted gap fills needed |
| 50–69% | Partial — Five W's present but thin; schedule a fill session |
| < 50% | Skeleton — doc exists but should not be considered reliable |

---

## How to run an audit

1. List the docs to audit (use `soul_list_documents` or target a namespace like `vault.*`)
2. For each doc, work through this checklist
3. Record scores and specific gaps in a gap-fill plan (see `gap-fill-plan.md` template)
4. Separate gaps into: "I can fill from code" vs "needs human input"
5. File work items for human-input gaps; handle code-derivable gaps in the same session

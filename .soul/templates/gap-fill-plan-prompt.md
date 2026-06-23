# Prompt — Work a Gap-Fill Plan

Use this prompt when you want Claude to execute against an existing gap-fill plan — either filling specific gaps, updating the plan after your answers, or producing a new plan from a fresh audit.

---

## Prompt — Fill gaps from the plan

```
Open .soul/templates/gap-fill-plan.md and work through the open gaps for [audit section / Soul ID / category].

For each gap you address:
1. Update the Soul doc with the new or improved content
2. Mark the gap as Filled in the plan table
3. If filling reveals new gaps, add them to the plan as new rows

Only work Code-category gaps unless I have answered the Human ones below.

[Optional: paste your answers to Human gaps here]
```

---

## Prompt — Record answers to Human gaps

```
I have answers to some of the open Human gaps in .soul/templates/gap-fill-plan.md. Update the relevant Soul docs and mark those gaps as Filled.

[soul.id] — [gap description]:
> [your answer]

[soul.id] — [gap description]:
> [your answer]

After updating, tell me which gaps remain open and whether any of my answers revealed new gaps to add.
```

---

## Prompt — Generate a new gap-fill plan from an audit

```
Run the audit rubric at .soul/templates/audit-rubric.md against [namespace or list of Soul IDs] and produce a new dated section in .soul/templates/gap-fill-plan.md.

Format it using the table template in that file. Categorise every gap as Code / Human / Risk.
Include a Notes section at the bottom with any cross-cutting observations.

Do not fill any gaps yet.
```

---

## Examples

**Fill only Code gaps for vault:**
```
Open .soul/templates/gap-fill-plan.md and work through all open Code-category gaps in the vault.* audit section. Update the Soul docs and mark each gap Filled as you go.
```

**Answer Human gaps after a design conversation:**
```
I have answers to some open Human gaps in .soul/templates/gap-fill-plan.md. Update the relevant Soul docs and mark them Filled.

vault.tenant.provision — Is admin-gated activation permanent?:
> Yes, permanent. We want manual approval for every coach. Auto-provisioning may come later when Stripe subscription billing is wired up, but for now it stays manual.

vault.secret.delete — Is the 90-day soft-delete window intentional?:
> Yes. I want a recovery window in case a key is deleted by mistake. We will not be purging on delete.
```

**Fresh plan after adding a new feature:**
```
We just implemented the auth.clerk.webhook flow. Run the audit rubric against auth.clerk.webhook and add a new dated section to .soul/templates/gap-fill-plan.md with all identified gaps categorised.
```

---

## Tips

- Always scope to a specific audit section or Soul ID — the plan will grow over time
- Paste Human answers inline in the prompt rather than describing them separately — Claude can update multiple docs in one pass
- After filling Code gaps, re-run the rubric on the same docs to confirm the score improved
- Risk-category gaps should be filed as work items if they represent actual bugs, not just doc gaps

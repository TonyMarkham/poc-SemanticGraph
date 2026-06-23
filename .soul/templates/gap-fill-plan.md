# Soul Doc Gap-Fill Plan

## How to use this file

After running an audit (see `audit-rubric.md`), record findings here.
Each audit run gets its own dated section. Copy the table template, fill it in, then work through the gaps.

Gaps are categorised:
- **Code** — answerable by reading the codebase; Claude can fill these without human input
- **Human** — requires design decisions, business context, or intent that only the developer knows
- **Risk** — a potential bug, missing guard, or unhandled failure mode worth tracking as a work item

---

## Template (copy for each audit run)

```markdown
## Audit — [namespace] — [YYYY-MM-DD]

**Docs audited:** [list Soul IDs]
**Auditor:** [Claude / Tony / both]

| Soul ID | Gap | Category | Status |
|---|---|---|---|
| `x.y.z` | [specific question or missing W] | Code / Human / Risk | Open / Filled / Tracked |

### Notes
[Any cross-cutting observations that apply to multiple docs]
```

---

## Audit — vault.* — 2026-04-11

**Docs audited:** `vault.provider`, `vault.secret.get`, `vault.secret.set`, `vault.secret.delete`, `vault.secret.list`, `vault.tenant.provision`, `vault.tenant.delete-key`, `vault.admin.list-keys`, `vault.admin.delete-key`

**Auditor:** Claude (extrapolated from codebase)

| Soul ID | Gap | Category | Status |
|---|---|---|---|
| `vault.provider` | Why Azure Key Vault specifically over alternatives (AWS SM, HashiCorp, GCP)? | Human | Open |
| `vault.provider` | Is single-instance deployment a hard assumption, or does multi-instance boot need guarding? | Human | Open |
| `vault.provider` | Service principal secret rotation procedure without downtime | Human | Open |
| `vault.secret.get` | Plan for in-memory key rotation without full orchestrator restart | Human | Open |
| `vault.secret.get` | Retry/backoff policy when vault unreachable at boot — hard exit intentional? | Human | Open |
| `vault.secret.get` | `unwrap_or_default()` returns empty string for missing value — should this be validated before use as an encryption key? | Risk | Open |
| `vault.secret.set` | Re-provisioning: should the existing vault key be reused or a new one generated? | Human | Open |
| `vault.secret.set` | First-boot race condition — two simultaneous orchestrator instances both bootstrap | Risk | Open |
| `vault.secret.delete` | Is the 90-day soft-delete window intentional as a recovery mechanism, or just the Azure default? | Human | Open |
| `vault.secret.delete` | Is there a documented recovery procedure for accidentally deleted keys? | Human | Open |
| `vault.secret.delete` | Purge policy after offboarding confirmed complete | Human | Open |
| `vault.secret.delete` | Pre-migration `0003` tenants may have no `vault_jwt_key_name` — hard-delete handler attempts to delete it regardless | Risk | Open |
| `vault.secret.list` | Scale: paginated list may be slow with hundreds of tenants; prefix-filter strategy planned? | Human | Open |
| `vault.secret.list` | Soft-deleted secrets not visible in list — no UI path to view or recover them | Human | Open |
| `vault.tenant.provision` | Is admin-gated activation permanent or a temporary placeholder until billing automation? | Human | Open |
| `vault.tenant.provision` | No rollback of vault key if directory or DB creation fails — orphan key left in vault | Risk | Open |
| `vault.tenant.provision` | JWT key and onboarding key creation — when and how are these provisioned? | Human | Open |
| `vault.tenant.delete-key` | Full intended offboarding sequence (suspend → delete key → hard delete?) | Human | Open |
| `vault.tenant.delete-key` | Is there a documented key recovery procedure using the Azure soft-delete window? | Human | Open |
| `vault.tenant.delete-key` | DB file remains on disk after key deletion — is a separate cleanup step planned? | Human | Open |
| `vault.tenant.delete-key` | Only deletes DB key — JWT and onboarding keys handled elsewhere or by hard-delete handler? | Code | Open |
| `vault.admin.list-keys` | Post-key-deletion: tenant query filters `vault_db_key_name != ''` so key-deleted tenants disappear from ownership map — intentional? | Human | Open |
| `vault.admin.list-keys` | Vault page not refreshed on `TenantKeyDeleted` WebSocket event — stale view acceptable? | Human | Open |
| `vault.admin.delete-key` | **No guard against deleting `platform-db-key` or `platform-admin-token` by name** — catastrophic if triggered | Risk | Open |
| `vault.admin.delete-key` | No audit record written to `platform.db` for vault key deletions | Human | Open |
| `vault.admin.delete-key` | Full set of orphan-creation scenarios beyond failed provisioning | Code | Open |

### Notes

- The highest-priority risk is `vault.admin.delete-key` — a platform admin can currently delete `platform-db-key` via the Vault page UI with no additional guard. This should be tracked as a bug.
- The `vault.secret.get` empty-value risk is low probability (requires a malformed secret to already be in Azure) but the consequence is silent DB corruption.
- The pre-migration `0003` JWT key risk depends on whether any tenants were provisioned before that migration ran. If the platform is greenfield, this is a non-issue.
- Several "Human" gaps around offboarding sequence and key recovery are related — a single offboarding design doc would resolve most of them.

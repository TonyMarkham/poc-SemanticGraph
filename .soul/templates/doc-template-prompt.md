# Prompt — Create a Soul Doc

Use this prompt when you want Claude to write a new Soul document for a feature, interaction, or concept.

---

## Prompt

```
Using the Soul doc template at .soul/templates/doc-template.md, create a new Soul document for [feature/interaction/concept].

Soul ID: [dot.separated.id]
Kind: [concept | interaction | policy | decision]

Context:
- [Any relevant background — what the feature does, where it fits in the architecture]
- [Any related Soul IDs already documented]

Derive what you can from the codebase. For the Five W's:
- Fill Who, What, When, and Where from the code
- For Why, use FOUNDATION.md and any related .docs files as context before inferring
- Flag anything you cannot determine in the "Needs elaboration" section as specific, actionable questions

Place the file at .docs/[namespace]/[name].md and annotate the relevant Rust and C# code with #[soul(...)] and [Soul(...)] respectively.
```

---

## Example

```
Using the Soul doc template at .soul/templates/doc-template.md, create a new Soul document for the Stripe Connect onboarding flow.

Soul ID: stripe.connect.onboarding
Kind: interaction

Context:
- Coaches complete Stripe Connect onboarding before they can receive payments
- The flow is initiated from the platform dashboard and handled by the orchestrator
- Related Soul IDs: none yet

Derive what you can from the codebase. For the Five W's:
- Fill Who, What, When, and Where from the code
- For Why, use FOUNDATION.md and any related .docs files as context before inferring
- Flag anything you cannot determine in the "Needs elaboration" section as specific, actionable questions

Place the file at .docs/stripe/connect-onboarding.md and annotate the relevant Rust and C# code with #[soul(...)] and [Soul(...)] respectively.
```

---

## Tips

- Provide the Soul ID up front — Claude will use it for both the doc frontmatter and the code annotations
- The more context you give, the fewer gaps end up in "Needs elaboration"
- If you already know the Why, include it — Claude cannot always infer design intent from code alone
- You can scope to doc-only (no annotations) by adding "do not add code annotations yet"

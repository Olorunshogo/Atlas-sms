# Atlas - Claude Instructions

Project-specific additions to the global instructions. Commit message format, branch naming, and PR rules are already fully defined in [CONTRIBUTING.md](./CONTRIBUTING.md) - this file only adds what CONTRIBUTING.md doesn't cover.

---

## Git Commits

- Always sign commits (`git commit -S`, or rely on `commit.gpgsign`/SSH signing already configured) unless explicitly asked for an unsigned commit.
- Every commit message is a single line, using the `<prefix>: <description>` format and prefix list in [CONTRIBUTING.md](./CONTRIBUTING.md) - no body, no bullet points, no multi-line descriptions.

---

## Comment Style

Reasonable comments are welcome - comment on non-obvious WHY, not WHAT. Use
these conventions where a comment is actually warranted, not as decoration
on every few lines - a file with a heading divider every five lines is
noise, not structure.

**Single-line:** `//` - Rust (`atlas-backend`), TypeScript/JavaScript, and inside Vue's `<script>`/`<script setup>` blocks (`atlas-marketing`, `atlas-pos`, `@atlas/ui-core`, `@atlas/ui-web`, `@atlas/ui-app`).

**Section-heading comments (`//` with `=` signs) - a heading hierarchy, use only the depth a file actually needs:**

- `// === Section` - top-level divider (e.g. `// === Types`, `// === Helpers`, `// === Component`). Most files need at most a handful of these.
- `// == Subsection` - one level under a `===` section, only when that section is genuinely large enough to need internal grouping.
- `// = Sub-subsection` - one level under a `==` subsection. Rare - if a file needs this much nesting, consider whether it should be split into multiple files instead.
- Plain `//` is never a heading - it's an ordinary inline comment, same as always.

**Multi-line comments use proper doc-comment syntax, not several `//` lines glued together:**

```ts
/**
 * Explanation that genuinely needs more than one line - the WHY, not a
 * restatement of the code below it.
 */
```

- Rust, TypeScript, JavaScript (anywhere outside JSX markup): `/** ... */` with each continuation line prefixed `* ` and aligned under the opening `/**`. Don't stack bare `// line one` / `// line two` comments to fake a paragraph, and don't write a single-line-style `/* all of this on one line */` for something that's actually multiple sentences of reasoning - use the JSDoc block shape above once it's genuinely multi-line.
- JSX/TSX markup (only relevant if React ever reappears in this repo - there is no React code in the tree currently): `{/* ... */}` - the braces are required there because a JSX child must be a valid JS expression. `{/* */}` is invalid everywhere else - never use it in Rust or plain TS/JS logic.
- Vue `<template>` blocks (`.vue` files across `atlas-marketing`, `atlas-pos`, `@atlas/ui-core`, `@atlas/ui-web`, `@atlas/ui-app`): HTML-style `<!-- ... -->`. Vue `<script>` blocks use normal `//` / `/** */`, not JSX or HTML syntax.

**No em dash in any comment**, single-line or block - same rule as markdown/commits/PRs elsewhere in this file. Use a plain hyphen or rewrite the sentence.

---

## Where to look before building

Replaces the three navigation-only files that used to sit in
`docs/frontend/design/getting-started/` (`INDEX.md`, `README.md`,
`START_HERE.md`), each of which existed only to explain the reading order
of the others.

**Before writing any code:** [docs/atlas/CONTEXT.md](docs/atlas/CONTEXT.md).
The locked architecture decisions are settled. Do not re-litigate or work
around them.

**Before building UI:** [docs/atlas/SystemDesignDocument.md](docs/atlas/SystemDesignDocument.md)
and [docs/RolesAndPermissions.md](docs/RolesAndPermissions.md). No component
hardcodes a Tailwind colour; use the tokens in `libs/shared/theme`.

**Before touching the schema or auth:** the migrations under
`apps/atlas-backend/migrations/` are the source of truth for schema,
ahead of any SQL excerpt in a doc.
[docs/atlas/auth-flow.md](docs/atlas/auth-flow.md) is the auth design.
Read the handlers under `apps/atlas-backend/src/auth/` before treating that
doc as current: the doc lags the code in places.

**Package placement:** if a second app could plausibly need it, it does not
live in an app. `@nx/enforce-module-boundaries` enforces this, so getting
it wrong is a lint failure rather than a review comment.

---

## Locked files: `docs/jottings/`

**`docs/jottings/` is the owner's personal working material. Never edit,
rewrite, reorganize, rename, or delete anything in it.**
`docs/jottings/TechPRD.md` is the product PRD and is the most important
instance of this rule: read it freely, cite it freely, change it never.

This holds even when an instruction sounds like it covers the whole repo. A
generic request such as "check the docs", "update the docs", "clean up
documentation", "fix the broken links", "align the docs with the new
decisions", or a wide refactor **does not** authorize touching anything
under `docs/jottings/`. Only an explicit, by-name instruction to change
that specific file does.

If a docs-wide task would otherwise sweep these files up, skip them and say
so in the summary rather than silently including or silently omitting them.
If a link in a jottings file becomes stale because a file outside jottings
moved, leave it and report it; do not fix it in place.

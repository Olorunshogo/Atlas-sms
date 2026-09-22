# Contributing

This document defines how work happens in this repository: branch naming, commit messages, pull request rules, and the automated checks that enforce them.

---

## Branch Naming

Branch names are validated automatically (see [Automated Checks](#automated-checks)) against this pattern:

```
^(main|develop|live)$|^(feature|fix|refactor|hotfix|release|conflict|chore)/.*$
```

This means every branch must be either:

- `main`, `develop`, or `live` (the long-lived branches), or
- `<type>/<topic>`, where `<type>` is one of the values below and `<topic>` is a short, kebab-case description of the work.

| Type        | Use for                                                                      |
| ----------- | ---------------------------------------------------------------------------- |
| `feature/`  | New functionality or pages                                                   |
| `fix/`      | Bug fixes                                                                    |
| `refactor/` | Restructuring code with no behavior change                                   |
| `hotfix/`   | Urgent fixes, usually branched from `main` and merged back quickly           |
| `release/`  | Preparing a release cut                                                      |
| `conflict/` | Isolated branch for resolving a specific merge conflict                      |
| `chore/`    | Maintenance work with no user-facing behavior change (deps, config, tooling) |

Examples:

```
feature/atlas-store-transfers
fix/atlas-backend-tenant-lookup
refactor/atlas-ui-vue-migration
hotfix/broken-tenant-lookup
release/v1.1.0
conflict/develop-main-merge
chore/mcp-server-config
```

Branches that do not match this pattern are rejected by the pre-push hook (`validate-branch-name`).

---

## Commit Messages

Commit messages use a `<prefix>: <description>` format, based on [Conventional Commits](https://www.conventionalcommits.org/), matching the branch types above with `feature` shortened to `feat` for the commit prefix itself:

| Prefix      | Use for                                                 |
| ----------- | ------------------------------------------------------- |
| `feat:`     | New functionality or pages                              |
| `fix:`      | Bug fixes                                               |
| `refactor:` | Restructuring code with no behavior change              |
| `hotfix:`   | Urgent production fixes                                 |
| `release:`  | Release preparation (version bump, changelog)           |
| `chore:`    | Maintenance work with no user-facing behavior change    |
| `test:`     | Adding or updating tests only                           |
| `style:`    | Formatting, whitespace, or purely cosmetic code changes |

### Rules

- One prefix per commit. If a commit touches more than one category, split it into multiple commits.
- The description is short, plain, and in the imperative mood ("fix overflow", not "fixed overflow" or "fixes overflow").
- Do not use the em dash character anywhere in a commit message. Use a plain hyphen or rewrite the sentence.
- Do not add `Co-Authored-By` lines or any co-authorship attribution.
- Keep the summary line under about 72 characters. If more detail is needed, add it as a separate paragraph in the commit body, not by stretching the summary line.

### Examples

```
feat: scaffold atlas-marketing as a Nuxt 4 app
fix: correct tenant resolution in atlas-backend's TenantTx
chore: wire GitHub and Postgres MCP servers
refactor: convert atlas-ui packages to Vue peer deps
```

### Generating a commit message with AI

When asking an AI assistant to write a commit message, give it exactly this prompt so the result matches the rules above:

```
Give me a one-line commit message with the right prefix for the currently staged files.
Base it only on what the staged diff actually changed, not on unstaged or untracked changes.
Follow the "<prefix>: <description>" format and prefix list in CONTRIBUTING.md, no em dashes, no Co-Authored-By line.
```

The "staged only" instruction matters: a working tree often has more in progress than what is about to be committed, and the message must describe the commit being made, not the whole session.

---

## Pull Request Rules

- **One concern per PR.** A PR should represent one feature, one fix, or one cohesive refactor. Do not bundle unrelated changes.
- **Title** follows the same `<prefix>: <description>` format as commits, and should be understandable without opening the PR.
- **Description** should include:
  - What changed and why (the motivation, not just a restatement of the diff).
  - Any tradeoffs or follow-up work that was deliberately left out of scope.
  - A test plan: what was checked manually, and whether any automated tests were added or updated.
- **Screenshots or a short clip** are expected for any visual or UI change (`atlas-marketing`, `atlas-ui`, `landing-page`).
- **Base branch**: `feature/`, `fix/`, `refactor/`, `chore/`, `release/`, `conflict/` branches target `develop` unless the change is a `hotfix/`, which targets `main` directly.
- **Before opening a PR**, make sure `pnpm lint` and the relevant `pnpm build:*` script both pass locally.
- **Merging**: prefer squash merges so `main`/`develop` history stays one commit per PR, matching the commit message rules above.
- **Do not force-push** to `main` or `develop`. Both are protected by the pre-push hook, and only the repository owner can push to `main` directly.

---

## Direct Branch Merge (non-PR workflow)

For solo work where a full PR review cycle doesn't apply, merging a
`feature/`/`fix/`/`refactor/`/`chore/` branch straight into `develop` is
done manually rather than through a squash-merge PR. This bypasses the
"one concern per PR" review step above - only use it when that's an
acceptable tradeoff for the change.

```bash
# 1. Review what's actually about to be committed - don't blindly `git add -A`
git status

# 2. Stage and commit. One prefix per commit (see Commit Messages above) -
#    if the branch's work spans multiple categories, split into multiple
#    commits here rather than squashing everything into one.
git add <files>
git commit -S -m "<prefix>: <description>"

# 3. Fetch and update develop before merging into it
git fetch origin
git checkout develop
git pull origin develop

# 4. Merge the feature branch in (a real merge, not a squash - squash is
#    the PR-flow's job, described above)
git merge <feature-branch>

# 5. Verify before pushing to a shared branch
pnpm lint
pnpm build:ui          # or whichever pnpm build:* scripts the change touched

# 6. Push
git push origin develop
```

If the pre-push hook (branch-name validation, backend e2e tests,
protected-branch checks) rejects the push, fix the underlying issue - do
not `--no-verify` past it, same rule as everywhere else in this doc.

---

## Automated Checks

This repo enforces conventions with Husky git hooks, not just documentation:

- **`pre-commit`** runs `lint-staged` (ESLint + Prettier on staged files per `.lintstagedrc`).
- **`pre-push`** runs `validate-branch-name`, backend e2e tests, and protected-branch checks on `main` and `develop`: no deleting or force-pushing `develop`, no deleting or force-pushing `main`, and only the repository owner can push to `main` at all. It also rejects pushes that include media files over 100KB or any tracked file over 500KB, media belongs on a CDN, not in git.

If a hook rejects your commit or push, fix the underlying issue rather than bypassing it. Do not use `--no-verify` to skip these checks.

---

## Code Style

Beyond commit and branch conventions, code itself follows the project's existing patterns:

- **`atlas-marketing`, `landing-page`, `atlas-web`**: Nuxt 4 apps. Match the `app/{components,composables,layouts,pages,plugins,utils,models,types}` structure, Tailwind v4 via `@tailwindcss/vite`, Pinia for client state.
- **`atlas-ui/{core,web,app}`, `atlas-ui/{core,web,app}`**: Vue component packages (`@atlas/ui-app`/`@atlasui-app` for Ionic/mobile, `@atlas/ui-web`/`@atlasui-web` for shadcn-vue/reka-ui based web components), built with `tsup`.
- **`atlas-backend`, `atlas-backend`**: Rust (Axum), each its own Cargo workspace member, not Nx-native.
- `atlas-*` paths are reference code being retired incrementally as Atlas gets its own equivalent - see the README's "Why `atlas-*` is still here". Don't add new Klassivox-product features there; that product has its own separate repository.
- No `any` in TypeScript except at genuinely unavoidable boundaries (documented inline when used).
- Always annotate state explicitly (`ref<boolean>(false)`, not `ref(false)`), even when TypeScript could infer it - see `docs/repo/CONTEXT.md`'s "State typing" entry for the full rule and reasoning.
- Run `pnpm format` before committing if you are unsure whether Prettier will change your files (`lint-staged` also does this automatically on commit).

---

## Getting Help

If a convention here is unclear or seems to conflict with the code you are looking at, treat this document as the source of truth and flag the discrepancy in your PR description rather than guessing.

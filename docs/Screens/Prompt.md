# Klassivox — session history vs repo state

Reconstructed from every Claude Code transcript found across `.claude1`, `.claude2`, `.claude3` for this project (7 real sessions, spanning 2026-09-10 through 2026-09-17, under path spellings `Zunetra-Klassivox`, `Zunetra-03-Klassivox`, `Zunetra-03_Klassivox`; an eighth directory, `React-Klassivox`, has no transcripts left — likely an earlier React-only phase whose history is gone). Compared against the actual repo on disk (`git log`, `git ls-files`, current source tree) as of 2026-09-19.

## No stack pivot found

The `React-Klassivox` directory name suggested an earlier React phase, but its transcripts are empty, so there is no evidence either way. Every real transcript found already shows the current stack: Nx monorepo, Nuxt 4 web app, Rust/Axum backend, Postgres with RLS multi-tenancy. Treat this as the stack going forward.

## What was discussed and built across sessions

**2026-09-10** — Backend foundation session. Built, tested, and verified working: `AppState`, `TenantTx` (the tenant-scoped transaction extractor that is the whole enforcement mechanism for RLS), `EmailProvider` trait + log implementation, `POST /api/v1/auth/signup` (full validation, argon2 hashing, OTP issuance, `organization`/`school` fork), `AdminTx` (unscoped sibling for pre-tenant handlers). `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` all green. Backend dockerised against the existing compose stack (ports shifted to 5433/6380 to coexist with `debridgers-repo`).

**2026-09-16 (two sessions)** — Extended the backend: soft-delete semantics decided (soft delete is the default; only a super admin can hard-delete; deleting an organization cascades to its schools; restore brings them back together), UUID vs sequential ID question resolved in favor of UUID, step-up confirmation added to the refund endpoint (`payments.refund`), work started on the student-enrollment chain to exercise the refund flow end-to-end, integration tests written against curl/live DB rather than mocks, a `nx affected` scope check across all 12 workspace projects.

**2026-09-16/17** — Product-shape conversation: how to model Nigerian school structures where graduation points vary (Primary 5 vs 6, SS2 exit, etc.). Decision: schools configure their own grade hierarchy per-school in settings (creche/nursery/primary/secondary as an editable ladder) rather than a fixed national structure, so the platform works for Montessori, private, and government schools alike. User asked for this split into `docs/features/v1.md` (now) and `docs/features/v2.md` (later — e.g. a "record a transfer" workflow instead of a dedicated transfer feature). **This doc split does not exist in the repo** — no `docs/features/` directory at all currently.

**2026-09-17** — Implemented the configurable grade hierarchy and a promotion endpoint. An assistant auto-committed this work as `bc39160` after a "go ahead" that was answering a broader recap, not authorizing a commit. You caught it and said: *"i did not ask you to commit, reset the commit and probably even deleting it self"* — asking to undo the commit but **keep the file changes** as uncommitted work. The assistant confirmed it would remove only the commit, not the work.

## What's actually in the repo now

- **One commit** on `main` (`40a7e32`, "Initial commit", dated 2026-08-04) plus one doc-only commit (`bfb1c5f`, "Writing a more comprehensive README"). `origin/develop` is nearly identical to `main` (4 lines of diff, doc tweaks only).
- `git reflog` shows exactly one entry: `clone: from github.com:Zunetra/Klassivox.git`. This is a fresh clone with no local history beyond what's on the remote — meaning any uncommitted work from past sessions was **never pushed anywhere** and is gone from disk.
- `apps/atlas-backend/src/` currently contains only `main.rs` and `lib.rs`. **None of the 2026-09-10 through 2026-09-17 backend work is present**: no `state.rs`, `tenant.rs`, `auth/`, `providers/`, `error.rs`, `response.rs`, no soft-delete columns/migrations, no grade-hierarchy or promotion endpoint, no refund step-up.
- `docs/repo/NEXT.md` (as committed) still describes the **pre-2026-09-10** state: "atlas-backend is still a GET /health stub... no auth, no TenantTx" — this was true before the 09-10 session, false immediately after it, and the doc was never updated to catch up because the work behind it never landed.
- No `docs/features/` directory (the v1/v2 split from the 09-16/17 product conversation was never created).

## Drift / gaps to address

1. **The entire auth module (TenantTx, signup, providers) built and verified on 2026-09-10 needs to be rebuilt.** It compiled, passed tests, and passed clippy/fmt once — this is re-implementation, not new design work. Follow the plan already captured above.
2. **The soft-delete design (09-16) is a decision, not yet schema or code.** Cascade-delete/restore semantics for org → schools need migrations + handlers.
3. **The refund step-up and student-enrollment chain (09-16) never landed.**
4. **The configurable grade-hierarchy + promotion endpoint (09-17) is the most recently-lost work** — it existed as uncommitted changes at the end of that session and is not recoverable from git. This is the highest-priority rebuild since it was the newest product decision (per-school configurable ladder to support creche/nursery/primary/secondary variability).
5. **`docs/repo/NEXT.md` is stale** and should be rewritten once the above is rebuilt, so it stops describing pre-09-10 state as current.
6. **`docs/features/v1.md` / `v2.md` were requested but never created** — worth creating now while the 09-16 conversation is still fresh in this doc, to formally record the v1 (per-school configurable hierarchy, "record a transfer" in v2) decision.
7. **Process note, not code**: the near-loss here happened because an assistant committed without explicit authorization, and the subsequent "undo" left the work sitting only as uncommitted local changes in a directory that was later replaced. Commit early once work is verified, rather than leaving large verified chunks uncommitted between sessions.

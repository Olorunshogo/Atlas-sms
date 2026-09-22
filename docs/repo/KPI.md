# CTO KPIs - Klassivox

The single KPI doc. Tagging system: **Attainable**, **Stretch**,
**Unreasonable**, with reasoning so each can be revisited as the team and
product grow. The separate submitted-upward summary that used to live in
`CTO-KPI-SUMMARY.md` is folded into "This quarter, condensed" at the bottom
rather than maintained as a second file that drifts from this one. Current
state and the next concrete task live in [NEXT.md](NEXT.md); settled
decisions live in [CONTEXT.md](CONTEXT.md).

Baseline assumption, and it matters a lot here: **Klassivox is pre-code on
every product module.** `atlas-backend` is a health-check-only stub;
`atlas-web` and `landing-page` are Nuxt scaffolds with no real pages.
Several KPI categories are marked Unreasonable purely because the thing
they measure does not exist yet, not because the goal is bad. The exception
is `@atlasui-core`, where real component infrastructure (dialog
system, form primitives) already exists, which is why component test
coverage below is Attainable rather than Unreasonable.

What changed on 2026-07-31: the schema layer is no longer speculative. Six
migrations exist, have been applied to a live Postgres, and RLS tenant
isolation has been verified to fail closed. That moves several Security and
Data Integrity rows from aspiration to enforcement.

---

## 1. Engineering Delivery

| KPI                                                                                                    | Tag          | Why                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------ | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Ship 1-2 completed backlog items/week                                                                  | Attainable   | Matches solo/small-team velocity once items are scoped to endpoint+migration+frontend granularity, as [NEXT.md](NEXT.md) does for Auth.                                                                      |
| Zero P0 production incidents/month                                                                     | Attainable   | Trivially true right now - there is no production traffic yet. Becomes a real KPI once `atlas-web`/`atlas-backend` are live for real users.                                                          |
| Full CI pipeline (frontend + backend jobs) green on every merge                                        | Attainable   | Plan already exists in [CONTEXT.md](CONTEXT.md) decision 12; just needs `.github/workflows/ci.yml` written per its rollout order.                                                                            |
| 90%+ TypeScript strict-mode coverage, zero `any` in new code                                           | Attainable   | Already a stated repo convention (`coding-style` skill); enforcement, not new work.                                                                                                                          |
| `cargo clippy`/`cargo fmt --check` clean on every backend merge                                        | Attainable   | Same reasoning, Rust side - low-effort once CI job 2 (backend) exists per [CONTEXT.md](CONTEXT.md) decision 12.                                                                                              |
| 100% of new endpoints covered by at least one integration test                                         | Stretch      | Achievable if built in from the start (unlike Debridgers, which had to retrofit) - but competes with feature velocity early on.                                                                              |
| `@atlasui-core`'s stateful components (dialog stack, dialog submission) have a green Vitest suite | Attainable   | Infra exists (`vitest.config.ts`, `@vue/test-utils`); `dialog-provider.spec.ts` is written but not yet green (`reka-ui`/jsdom `Presence` timing) - this KPI tracks getting it green, not building new infra. |
| Full test suite runtime under 5 minutes                                                                | Unreasonable | Premature - there's near zero test suite yet. Revisit once Phase 1 modules exist.                                                                                                                            |

---

## 2. Product / Feature Completion

| KPI                                                                     | Tag          | Why                                                                                                                                                                                                                           |
| ----------------------------------------------------------------------- | ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ship Auth + User/Role Management end-to-end this quarter                | Attainable   | Bootstrap-order Module 1+2 per [ARCHITECTURE.md](ARCHITECTURE.md) Section 9; fully scoped in [auth.md](../backend/auth.md) and [kyc-schema-api.md](../backend/kyc-schema-api.md).                                             |
| Ship Student Information System + Basic Academic module this quarter    | Stretch      | Depends on Auth/Role landing first per the bootstrap order; realistic only if Module 1+2 don't slip.                                                                                                                          |
| Ship Attendance + Report Card entry/publishing this quarter             | Unreasonable | Two modules deep in the bootstrap order (Module 6, then report cards) - not realistic in the same quarter as Auth/Role/SIS from a standing start.                                                                             |
| One role (Admin) working end-to-end through the bootstrap chain         | Attainable   | Explicitly ARCHITECTURE.md's own stated approach: "Implement one role end-to-end first (Admin)" (Section 9) rather than all roles partially.                                                                                  |
| `atlas-admin` scaffolded and serving `platform_admin` review queues | Unreasonable | Not started, not scoped yet, and platform-role tooling is lower priority than the customer-facing bootstrap chain per ARCHITECTURE.md.                                                                                        |
| `atlas-mobile` scaffolded                                           | Done         | App shell scaffolded (Vite + Vue 3 + Ionic Vue), wired into `@atlasui-core`/`ui-app` and the build pipeline. No native platforms or real screens yet - that's still unscheduled, no near-term driver to pull it forward. |

---

## 3. School/Business Metrics

| KPI                                                              | Tag          | Why                                                                                                                    |
| ---------------------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------- |
| First school fully onboarded (org created, students enrolled)    | Unreasonable | Blocked entirely on Auth/Role/SIS shipping first - nothing to onboard a school into yet.                               |
| Fee collection (Module 8) processing a real Paystack transaction | Unreasonable | Deep in the module list; also blocked on org/school entities existing.                                                 |
| Waitlist signups (landing-page)                                  | Attainable   | Fully scoped in [waitlist.md](../backend/waitlist.md); independent of the backend bootstrap chain - can ship any time. |

Debridgers' market-growth metrics (agent counts, city presence) have no Klassivox equivalent at this stage - there is no customer-facing product to grow yet. Revisit this section entirely once Phase 1 MVP ships.

---

## 4. Reliability / Ops

| KPI                                          | Tag          | Why                                                                                                                       |
| -------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------- |
| 99.9% API uptime                             | Unreasonable | No production API serving real traffic yet - premature by definition.                                                     |
| Monthly verified-restorable database backup  | Attainable   | Low-effort, high-value discipline; can start the moment a real Postgres instance holds real data.                         |
| Full disaster-recovery runbook with <1hr RTO | Unreasonable | Requires infra investment that doesn't match a single-Postgres-instance early-stage setup, same reasoning as Debridgers'. |

---

## 5. Security & Data Integrity

| KPI                                                                              | Tag          | Why                                                                                                                                                                                  |
| -------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| No plaintext credentials or secrets committed to the codebase                    | Attainable   | Basic hygiene, enforceable from day one.                                                                                                                                             |
| Formal security review before KYC/payment features ship                          | Stretch      | Worth a deliberate pass given student PII and money are both in scope (see [kyc.md](../backend/kyc.md) Section 7 on NDPR), but depends on there being something built to review yet. |
| NDPR-compliant handling of minors' data verified before any student record ships | Attainable   | Non-negotiable per [kyc.md](../backend/kyc.md) Section 7 - build it in from the first Student Information System commit, not retrofit later.                                         |
| SOC2 / formal compliance certification                                           | Unreasonable | Requires audit/process maturity far beyond a pre-launch team - revisit once there's real transaction volume driving it.                                                              |

---

## Category definitions

- **Attainable** = achievable now with only the scoped work already visible in this repo's docs ([NEXT.md](NEXT.md), [TechPRD.md](../jottings/TechPRD.md), [ARCHITECTURE.md](ARCHITECTURE.md)).
- **Stretch** = plausible this year/quarter, depends on prioritization tradeoffs against other Attainable items.
- **Unreasonable** = either blocked on earlier bootstrap-chain work that hasn't landed yet, or requires infrastructure/scale that doesn't exist. Not "bad" KPIs - wrong horizon for today.

---

## This quarter, condensed

The submitted scorecard. Same content as above, reduced to what someone
outside engineering needs.

1. **Bootstrap chain.** Auth and User/Role Management shipped end to end
   for one role (Admin), rather than every role half-built.
2. **Delivery predictability.** 1-2 completed backlog items per week.
   Completed means deployed to a real preview environment and off the
   backlog, not "in progress."
3. **CI exists.** A required check blocking merges on a broken build.
   Currently zero CI, only local Husky hooks, which are opt-out by design.
   Target is tiers 1 and 2 of the pipeline in [CONTEXT.md](CONTEXT.md)
   decision 12, required on `develop` and `main`.
4. **Security and data safety baseline.** No plaintext credentials
   committed, and NDPR-compliant handling of minors' data designed in from
   the first Student Information System commit rather than retrofitted.
   Partly banked already: tenant isolation is enforced by the database
   itself, verified to fail closed with no tenant context set.
5. **Waitlist growth.** Landing-page waitlist live and collecting signups.
   Independent of the bootstrap chain, so it runs in parallel.
6. **Shared UI discipline.** `@atlasui-core`'s stateful components
   have a green, CI-checked suite before any page consumes them. A
   regression there fans out to every downstream module. Now also enforced
   structurally: `@nx/enforce-module-boundaries` makes an app defining its
   own component a lint failure.

**Deliberately not on the scorecard yet:** market and business metrics
(schools onboarded, fee volume, MRR), since there is no customer-facing
product to measure; uptime SLAs and disaster-recovery targets, premature at
zero production traffic; `atlas-admin` and `atlas-mobile` progress,
both explicitly behind the bootstrap chain.

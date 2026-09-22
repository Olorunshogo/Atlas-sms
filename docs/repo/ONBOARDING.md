# Klassivox - Onboarding

How anyone joining Klassivox gets brought up to speed: staff, investors,
partners, advisors, and pilot schools. Each audience needs a different
subset in a different order, so this doc is a shared foundation plus one
track per audience.

Developer setup (install, commands, scaffolding a package) is **not** here.
That lives in [`README.md`](../../README.md). This doc is about people and
purpose.

> **Status.** Part 1 (the shared foundation) is drafted from the product
> direction already visible across the repo and needs founder sign-off
> before it is used externally. Part 2's tracks are structurally complete,
> but anything involving a number (traction, pricing, funding, headcount)
> is deliberately left blank rather than invented. See Part 4.

---

## Part 0: Which track are you on

| You are                        | Read                 | Time                   |
| ------------------------------ | -------------------- | ---------------------- |
| Joining engineering            | Part 1, then Track A | Half a day             |
| Joining ops, support, or sales | Part 1, then Track B | Two hours              |
| An investor or advisor         | Part 1, then Track C | One hour               |
| A partner or integrator        | Part 1, then Track D | One hour               |
| A pilot school going live      | Track E only         | Guided, not self-serve |

Everyone reads Part 1. Nobody needs a track that is not theirs.

---

## Part 1: The shared foundation

Five things get confused with each other constantly, so they are defined
before they are stated.

| Term            | What it actually is                                                                  |
| --------------- | ------------------------------------------------------------------------------------ |
| **Aim**         | The single sentence answer to "why does this company exist at all". Does not change. |
| **Vision**      | The world as it looks if we succeed. Aspirational, undated, describes an end state.  |
| **Mission**     | What we concretely do to get there. Active, describes work, not outcomes.            |
| **Objectives**  | Measurable checkpoints on the way. Dated, falsifiable, replaced as they are hit.     |
| **Core values** | Constraints on how we work. Only real if they sometimes cost us something.           |

### Aim

To make running a school in Nigeria a solved problem rather than a
paperwork problem.

### Vision

A Nigeria where every school, from a single classroom to a hundred-school
group, runs on one system that its staff trust more than paper. Where an
administrator spends their time on students rather than spreadsheets. Where
a parent knows how their child is doing without chasing a teacher. Where a
state can see its schools clearly, because the data was captured properly
in the first place.

Held to an international-school standard, because the schools we serve
compete with international schools, and because we intend to operate
outside Nigeria eventually.

### Mission

To replace the spreadsheets, paper registers, and disconnected tools
Nigerian schools run on today with one system covering academics,
admissions, finance, timetabling, and communication, built for how a real
term actually goes rather than an idealized workflow.

Concretely that means:

- Serving a single school bootstrapping alone and an organization running
  many schools, from day one.
- Making the core loop fast enough that staff actually use it: enroll a
  student, take attendance, grade, publish a report card, collect a fee.
- Nigeria-first infrastructure, state by state, with every provider behind
  a swappable interface so no single vendor is load-bearing.

### Objectives

Ordered checkpoints, not a roadmap. Each is either hit or not.

1. **Foundation.** Architecture locked, schema verified, tenant isolation
   enforced by the database. **Done, 2026-07-31.**
2. **First real module.** Auth and User/Role Management working end to end
   for one role, on real infrastructure.
3. **First school.** One real school running one full term on Klassivox,
   including report cards and fee collection.
4. **Proof of repeatability.** Enough schools that onboarding is a process
   rather than a favour, and someone other than the founder can run it.
5. **Proof of the multi-school thesis.** Organizations with several schools
   using the org dashboard as their actual management surface.
6. **State-level presence.** Coverage in one state deep enough to be the
   default rather than an option.

Numbers and dates are deliberately absent. See Part 4.

### Core values

Not marketing words. Each one has a cost attached, which is what makes it
real.

1. **Faster than paper, always.** If a feature is slower than the paper
   register it replaces, it has failed regardless of how complete it is.
   _Cost: we cut features that demo well but slow down a Monday morning._
2. **Student data protection is not negotiable.** Most of our records
   belong to children. Isolation is enforced by the database and fails
   closed, so a mistake returns nothing rather than another school's
   students. _Cost: every tenant-touching request pays for a transaction._
3. **Trust through verification, not bureaucracy.** Verification exists to
   protect data and money, not to make onboarding painful. _Cost: we carry
   the complexity of progressive verification instead of one hard gate._
4. **Build the seam, ship one implementation.** Providers, currency, and
   locale are swappable by configuration. We do not build the second
   implementation until there is a real second case. _Cost: an interface
   where a direct call would have been shorter._
5. **One place for shared things.** A component or formatter exists once
   and every app uses it. This is enforced by tooling, not goodwill.
   _Cost: contributors cannot take the local shortcut._
6. **Say what is actually true.** Status is reported as it is, including
   what is unfinished. _Cost: our own docs say "not built" a lot._

---

## Part 2: Tracks

### Track A - Engineering

**Read in this order. The order matters.**

1. This document, Part 1.
2. [`CONTEXT.md`](CONTEXT.md), "Locked architecture decisions". Settled and
   closed. Understand them before proposing anything that contradicts them.
3. [`NEXT.md`](NEXT.md). What actually exists today versus what is
   specified. The gap is large and knowing it prevents wasted work.
4. [`ARCHITECTURE.md`](ARCHITECTURE.md). The technical design.
5. [`../jottings/TechPRD.md`](../jottings/TechPRD.md). The product. **Read
   only, never edit.**
6. `README.md` for setup, then get the backend running against a local
   Postgres.
7. The domain doc for whatever you are building:
   [`auth.md`](../backend/auth.md),
   [`kyc-schema-api.md`](../backend/kyc-schema-api.md),
   [`schools.md`](../backend/schools.md),
   [`billing.md`](../backend/billing.md).

**You are ready when** you can explain why a handler receives a `TenantTx`
rather than a pool connection, and why school slugs carry a state prefix.

### Track B - Klassivox staff (ops, support, sales)

1. This document, Part 1.
2. "Who we serve" below.
3. [`../jottings/TechPRD.md`](../jottings/TechPRD.md) sections 1 through 5,
   for the product and role model. Skip the rest.
4. [`kyc.md`](../backend/kyc.md) for what verification asks of a school and
   why, since this is the most common support question.
5. Shadow a real school onboarding before running one alone.

**You are ready when** you can explain to a school owner what Klassivox
does, what verification will require of them, and what it will not do yet,
without overpromising.

### Track C - Investors and advisors

1. This document, Part 1, especially Aim, Objectives, and Core values.
2. "Who we serve" and "The thesis" below.
3. "Where we are going" below.
4. On request: current objective status, honestly stated, from
   [`NEXT.md`](NEXT.md) and [`KPI.md`](KPI.md).

**The thesis, in short.** Nigerian schools run on paper and spreadsheets.
Existing software is either built for Western school systems and does not
survive contact with a Nigerian term, or is a local point solution covering
one function. The opportunity is one system covering the whole operational
loop, sold state by state, where the fee-collection relationship creates a
second business later. The defensibility is the data: once a school's
enrollment, attendance, and academic history live in Klassivox, switching
costs are a term's worth of records.

**What we are deliberately not claiming yet:** traction, revenue, retention,
or unit economics. Product code has not shipped. Anyone presenting numbers
for these is making them up.

### Track D - Partners and integrators

1. This document, Part 1.
2. "Who we serve" below.
3. The integration surface: every app talks to one backend through a typed
   client, and providers sit behind interfaces, so integrating a payment,
   messaging, or identity provider is an adapter rather than a fork.
4. Tenancy: one deployment serves every school, and a school is resolved
   from its subdomain. There is no per-customer instance to integrate
   against separately.

**Open:** there is no partner programme, commercial terms, or API access
tier defined yet. This track describes the technical shape only.

### Track E - Pilot schools

Guided by a Klassivox staff member, not self-serve. Order:

1. What Klassivox does for this school specifically, in their terms.
2. Verification: what documents are needed and why.
3. Data migration: what they have now and what comes across.
4. The core loop, with their own staff, on their own data.
5. What is not built yet, stated plainly before they commit.
6. Who to contact when something breaks.

**Step 5 is not optional.** A pilot school that discovers a gap after
committing a term's records is a worse outcome than one that declined.

---

## Who we serve

Every feature serves one of these. Know them.

- **Organization owner.** Runs one school informally, or a group of
  schools. Cares about visibility across schools, fee collection
  reliability, and staff accountability.
- **School admin and staff.** Day-to-day operators: admissions, attendance,
  grading, report cards, timetables. They touch the product every school
  day, and their trust is earned by being faster than paper.
- **Parent or guardian.** Wants to know how their child is doing without
  chasing a teacher, and to pay fees without queuing. Cares about clarity
  and low friction, not features.
- **Student.** The subject of most records, rarely a direct user early on.
- **Investor viewer.** Read-only financial visibility into one school.
- **Klassivox platform team.** Verification, support, incident response.
- **State ministries and boards.** Oversight of schools they supervise but
  do not own. Designed for, not yet built.

---

## Where we are going

Directional, not committed. Every horizon below needs founder sign-off
before external use.

**Near term.** The bootstrap chain: Auth, User and Role, Student
Information, Basic Academic, Attendance, Report Cards, parent portal. One
role end to end before breadth. Waitlist running in parallel.

**Three years.** A paid product for independent schools and small groups,
past the "will anyone pay for this" question. Billing, timetabling,
documents, and events shipped. Platform admin tooling staffed. Mobile
shipped for parents and staff. First multi-school organizations managing
centrally rather than school by school.

**Five years.** A default choice for school administration in Nigeria, with
real depth in several state markets rather than scattered customers.
Analytics mature enough to be a school's primary operational reporting
tool. A compliance record strong enough to be a selling point rather than a
legal minimum.

**Ten years.** Infrastructure rather than a tool: a data layer schools and
potentially ministries rely on for enrollment, attendance, and performance
visibility, plus a financial layer built on the fee-collection
relationship. Expansion beyond Nigeria, on the seams already built for it.

---

## Part 4: What this document still needs from the founder

Deliberately blank rather than invented. Each needs a decision, not a draft.

- [ ] **Sign-off on Aim, Vision, Mission, and Core values.** These are the
      founder's to set. What is written above is a proposal built from the
      product direction visible in the repo.
- [ ] **Dates and numbers on the six objectives.** Currently ordered but
      unquantified. "First school" is checkable; "first school by March" is
      a commitment.
- [ ] **Investor materials.** Track C has a thesis and no numbers, because
      there are none yet. Decide what is shared and when.
- [ ] **Partner programme.** Track D describes technical shape only. No
      commercial terms, API tiers, or partner definition exist.
- [ ] **Compensation, equity, and employment terms.** Not in this repo and
      probably should not be.
- [ ] **Code of conduct and escalation path.** Needed before the first
      non-founder hire, not after.
- [ ] **Which market after Nigeria, and on what signal.** The earlier draft
      of this doc named Ghana with no stated reason.

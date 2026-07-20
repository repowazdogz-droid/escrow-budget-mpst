# PROTOCOL.md — the escrow-budget protocol, in prose, and what a session type can hold

This document does two things. First (§1) it writes down the protocol of the existing
`escrow-budget` artifact in prose, extracted from that artifact's TLA+ and Lean sources, and
checks the prose against the TLA+. Second (§2–§4) it gives the multiparty-session-type (MPST)
encoding used here, and records exactly where the encoding and the TLA+ protocol do **not** line
up. Those non-overlaps are findings, not omissions.

The sibling artifact lives at `../escrow-budget/`. Primary sources read:
`../escrow-budget/spec/EscrowBudget.tla` (Floor A), `.../EscrowBudgetD.tla` (Floor D, crash),
`.../Recovery.tla`, and `../escrow-budget/lean/Escrow/{State,Protocol,DurableState,DurableProtocol}.lean`.

---

## 1. The protocol, in prose (extracted from the TLA+ / Lean)

### 1.1 Participants

A fixed, finite roster of **N symmetric replicas**. One replica, `Genesis`, initially holds the
entire cap as local escrow; every other replica starts at zero. There is **no coordinator** and
**no dedicated ledger role**: replicas are peers. (The task brief speculated about a
"coordinator / spenders" shape. The artifact has neither term. A "spender" is just a replica
performing a local charge. This is recorded here so the reader is not misled.)

Membership is static: replicas do not join or leave mid-execution.

### 1.2 State

Per replica: `escrow` (local spend rights, a `Nat`) and `spent` (consumed, a `Nat`). Each
transfer id `t` has a lifecycle `phase(t) ∈ {unsent, inflight, received}`, an `amt(t)`, and a
`dest(t)`. Under crash/recovery (Floor D / Lean F2) every replica additionally has a **durable**
copy of its state; the live copy is **volatile**.

### 1.3 Actions

There are exactly these actions (names as in `EscrowBudget.tla` / `Protocol.lean`):

- **Charge(r, a)** — replica `r` consumes `a` from its own escrow: `escrow[r] -= a`,
  `spent[r] += a`, guarded by `a ≤ escrow[r]`. **This is a purely local action. It sends no
  message.** It is the only action that increases `spent`.
- **SendXfer(i, j, t, a)** — replica `i` debits `a` from its escrow and emits a transfer message
  `{to: j, tid: t, amt: a}` onto the network. Guarded by `t` unused and `a ≤ escrow[i]`. Under
  the crash discipline the debit is made durable **before** the message is emitted (write-ahead
  debit).
- **Recv(m)** — the destination credits `amt` to its escrow once per `tid` (receiver-side dedup),
  recording `tid` as received. Under the crash discipline the dedup record is durable.
- **Drop(m)** — the network loses a message. The debited right stays counted as in-flight
  (permanently lost capacity), so it is safe.
- **Persist(r) / Crash(r)** — checkpoint volatile → durable, and revert volatile ← durable. A
  crash loses all un-persisted work.

The network is a **set** of messages: delivery may **reorder, duplicate, and lose**. `Recv` may
fire on any sent message any number of times; the dedup guard makes re-delivery a no-op.

### 1.4 The property the artifact proves

Aggregate authorised spending never exceeds the cap:

```
    Σ_r spent[r]  ≤  CAP      (all reachable states)
```

proved in Lean for all finite N, all non-negative amounts, all CAP, crash-free
(`reachable_safe`) and under disciplined global crash/recovery (`durable_reachable_safe`), with
axioms `[propext, Quot.sound]` only. The inductive certificate is `A ≤ CAP` where
`A = Σspent + Σescrow + InFlight`. The artifact explicitly proves **no** liveness, **no**
progress, and **no** availability (its own README and SCOPE.md say so).

### 1.5 Prose-vs-TLA+ check

The prose above matches `EscrowBudget.tla` and `EscrowBudgetD.tla`. No disagreement between this
prose and the TLA+ was found. Two points worth flagging, both consistent with the sources:

- Floor A (`EscrowBudget.tla`) uses **unit** transfers (`amt = 1`); Floor C/D and the Lean model
  generalise to arbitrary `Nat` amounts. The invariant shape is identical. Prose uses the
  general form.
- The real transfer is **fire-and-forget**: `SendXfer` emits a message and the sender never waits
  for, or hears, an acknowledgement. `Recv` is a separate, independent action at the receiver.
  There is no request/response round-trip in the TLA+.

---

## 2. The global type (MPST encoding of one transfer)

Standard MPST describes a protocol as a **global type** over a fixed set of roles, mechanically
**projected** to a **local type** per role. Here the session-typeable unit is a **single
transfer** between a sender `S` and a receiver `R`. Roles: `S`, `R` (mpstthree names them `A`,
`B`).

```
  G  =  S -> R : Transfer(u32) .
        R -> S : { Credit . end          // fresh tid: receiver credits once
                 | Refuse . end }         // duplicate tid: receiver dedups, refuses
```

`R` chooses the branch; the choice **is** the dedup decision.

### 2.1 A modeling addition, stated up front

The `R -> S : {Credit | Refuse}` response **does not exist in the TLA+/Lean protocol**, which is
fire-and-forget (§1.5). It is added here so the receiver's dedup decision becomes an observable
session choice and so the session has a two-party interaction to which progress/deadlock-freedom
applies. This is an honest enrichment of the protocol, not a faithful transcription. The
one-way faithful alternative, `G₀ = S -> R : Transfer(u32) . end`, is trivially deadlock-free and
carries no branch; it would demonstrate less. Both are legitimate; we implement the richer one
and flag the addition. See finding **F-M1** in §4.

---

## 3. Projections (local types)

Projection of `G` to each role (mechanical; this is what MPST gives for free):

```
  G ↾ S  =  S!R:Transfer(u32) . S?R:{ Credit.end | Refuse.end }
  G ↾ R  =  R?S:Transfer(u32) . R⊕S:{ Credit.end | Refuse.end }
```

In the Rust encoding these are the `EndpointS` / `EndpointR` `MeshedChannels` types in
`src/bin/transfer_normal.rs`:

```
  EndpointS = MeshedChannels< Send<u32, Recv<TransferOutcome, End>>, RoleB<RoleB<RoleEnd>>, NameA >
  EndpointR = MeshedChannels< Recv<u32, Send<TransferOutcome, End>>, RoleA<RoleBroadcast>, NameB >
```

`TransferOutcome` is the `{Credit, Refuse}` branch enum. The two endpoints' binary types are
**dual** by construction; `fork_mpst` accepts them only because they are dual. That duality check
**is** the projection-compatibility / well-formedness check in the Rust-MPST setting, performed
by the compiler on every build. A non-projectable global type produces non-dual endpoints and is
rejected (negative control 2; see `examples/nc2_not_projectable.rs`).

---

## 4. Where the MPST encoding and the TLA+ protocol do NOT overlap (findings)

These are the substantive results of this second method. Each is a real gap between what the
session type expresses and what the artifact's protocol is.

- **F-M1 — added acknowledgement.** The `Credit/Refuse` response is a modeling addition (§2.1).
  The real protocol has no ack. So the session's branch structure describes an interaction the
  deployed protocol does not perform.

- **F-M2 — reliable channels vs an unreliable network.** mpstthree's channels are reliable,
  ordered, in-process (crossbeam). Vanilla MPST assumes reliable ordered channels. The escrow
  protocol's network **reorders, duplicates, and loses** messages, and its safety argument is
  built precisely to survive that. The MPST encoding here does **not** model loss, duplication,
  or reordering at the transport. It therefore cannot witness the properties that the artifact's
  `recvd`/dedup and `Drop` machinery exist to defend. This is the largest non-overlap.

- **F-M3 — crash-stop vs crash-recovery.** The affine variant models a participant that crashes
  and **stops permanently**, with the peer safely aborting (§ `transfer_crash_stop.rs`). The
  escrow protocol's crash is a **recovery**: revert to a durable checkpoint and **continue**,
  governed by write-ahead debit and durable dedup. Crash-stop session typing has no counterpart
  for "revert and continue"; the recovery discipline that the Lean F2 proof is about is outside
  what this method expresses. See GAPS.md.

- **F-M4 — no genuine multiparty interaction.** Every message in the protocol is point-to-point
  (`S → R`). There is no three-party synchronisation anywhere. The "multiparty" machinery of MPST
  (multi-role projection, a global choreography across ≥3 roles) is therefore unused: each
  transfer is a two-party session. The distributed character of the protocol lives entirely in
  the **aggregate** of many independent binary transfers plus local charges, and that aggregate
  is exactly the budget bound the session type cannot see (F-M5).

- **F-M5 — a session is one bounded transfer; the system is an unbounded reactive aggregate.**
  A session type describes one transfer's finite interaction. The real system is the unbounded
  interleaving of arbitrarily many transfers and local charges across N replicas. MPST gives
  per-session guarantees (this transfer is deadlock-free); it does not compose them into a
  system-wide state invariant. The budget bound `Σspent ≤ CAP` is a property of that unbounded
  aggregate over `spent`, which is mutated by the **local, message-free** `Charge` action
  (§1.3). Session types govern communication; they are structurally blind to `Charge`.

The consequence, developed in COMPARISON.md: the two methods are complementary. MPST establishes
communication-structure properties (deadlock-freedom, communication safety, progress) that the
Lean/TLA+ artifact never claimed; the Lean proof establishes the quantitative budget bound that
MPST cannot express. Neither is a re-proof of the other.

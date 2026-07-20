# COMPARISON.md — two methods, honestly separated

This is the contribution. It states, property by property, what each method establishes about the
escrow-budget protocol, using three cells: **ESTABLISHED**, **NOT ESTABLISHED**, **N/A** (the
property is outside what the method can express at all). The headline is that the methods are
**complementary, not redundant**. This is not "independent confirmation of the same theorem",
because the two methods establish different theorems about different aspects of the protocol.

The three methods:

- **MPST (this artifact)** — multiparty session types with crash-stop, in Rust via mpstthree.
  Establishes communication-structure properties of the transfer session, statically / by typing
  (so: all executions), plus crash-stop behaviour.
- **Lean 4 (sibling)** — machine-checked, unbounded, all N. Establishes the aggregate budget bound.
- **TLA+ / TLC (sibling)** — bounded, exhaustive at small scale. Establishes the budget bound and
  its crash/recovery invariants on small instances.

## The table

| Property | MPST (this) | Lean 4 (sibling) | TLA+/TLC (sibling) |
|---|---|---|---|
| **Aggregate budget bound** `Σspent ≤ CAP` | **N/A** — session types do not express arithmetic state invariants; and the `Charge` that moves budget sends no message, so it is invisible to the type | **ESTABLISHED** — unbounded, all N, all amounts, all CAP (`reachable_safe`, `durable_reachable_safe`), axioms `[propext, Quot.sound]` | **ESTABLISHED (bounded)** — exhaustive over small instances |
| **Deadlock-freedom** of the transfer session | **ESTABLISHED** — by typing; a well-typed session cannot get stuck at a mismatched send/recv | **NOT ESTABLISHED** — the artifact proves no liveness/progress (its own SCOPE.md) | **NOT ESTABLISHED** — deadlock-freedom was not among the checked properties; TLC could check it on bounded instances, but the artifact does not |
| **Communication safety** (no reception error / no payload mismatch) | **ESTABLISHED** — by typing; every receive has a matching, correctly typed send | **N/A** — the Lean model has no typed-channel layer; a reception mismatch is not even expressible in it | **N/A** — same; `Recv` is a guarded state transition, not a typed channel |
| **Progress** (the session advances to `end` absent a crash) | **ESTABLISHED** — by typing, for the well-typed session | **NOT ESTABLISHED** — no progress claim | **NOT ESTABLISHED** — no progress claim |
| **Message reorder / duplicate / loss tolerance** | **N/A** — mpstthree channels are reliable and ordered; vanilla MPST assumes reliable ordered transport, so loss/dup/reorder are assumed away, not modelled | **ESTABLISHED** — the safety proof holds under arbitrary reorder/dup/loss (the network is a set; `Drop` is safe) | **ESTABLISHED (bounded)** — exhaustive over reorder/dup/loss interleavings at small scale |
| **Crash tolerance — communication** (a crash never deadlocks the peer or causes a reception error) | **ESTABLISHED (crash-stop)** — structural, all interleavings, at every protocol point (see caveat below) | **NOT ESTABLISHED** — no communication/liveness claim under crash | **NOT ESTABLISHED** as a communication property; crash is modelled only as a state revert |
| **Crash tolerance — budget** (`Σspent ≤ CAP` survives crash/recovery) | **N/A** — the budget bound is not expressible (row 1); and this is crash-*recovery*, which crash-*stop* typing does not model | **ESTABLISHED** — disciplined **global** crash/recovery, unbounded (`durable_reachable_safe`) | **ESTABLISHED (bounded)** — per-replica crash/recovery, exhaustive at small scale (`EscrowBudgetD`) |
| **Recovery discipline is load-bearing** (write-ahead debit, durable dedup) | **N/A** — no notion of durable vs volatile state, or of recover-and-continue, in crash-stop typing | **ESTABLISHED** — negative-control theorems break the proof if either is dropped | **ESTABLISHED (bounded)** — model-checked counterexamples (`EscrowBudgetD_LazyDebit`, `_VolatileRecvd`) |

## What MPST adds that Lean/TLA+ did not give

The sibling artifact is explicit that it proves **no liveness, no progress, no deadlock-freedom**
(README: "No liveness, no fairness, no progress"; SCOPE.md: "No liveness / progress"). MPST
supplies exactly those, for the transfer session, by construction:

- **Deadlock-freedom and communication safety** hold for any well-typed pair of endpoints, on
  every execution, checked by the Rust compiler. The negative controls show the check has teeth:
  an out-of-order or wrong-payload implementation (`nc1`) and a non-projectable global type
  (`nc2`) both fail to compile.
- **Progress** for the session follows from the same typing.

These are structural (all executions), not bounded. They are a genuinely different guarantee from
the budget bound, about a genuinely different aspect of the protocol (its communication skeleton).

## What Lean/TLA+ establish that MPST cannot

- **The budget bound `Σspent ≤ CAP`** is an arithmetic invariant over aggregate state. Session
  types do not range over that. The point is sharpened by `Charge`: the action that consumes
  budget carries **no message**, so it is not a session event at all. MPST could type every
  transfer perfectly and say nothing about whether the system overspends.
- **Reorder/duplicate/loss tolerance** and **crash/recovery budget safety** are established by the
  Lean/TLA+ work and are outside the reliable-channel, crash-stop model used here.

## Crash tolerance, compared precisely

The two methods both touch "crash", but mean different things.

- **MPST (crash-stop).** Injecting a crash at **every** protocol point (`transfer_crash_stop`)
  shows the surviving peer never deadlocks and never suffers a reception error. Five of the six
  survivor observations are a clean `AbortOnRecv` (the crash appears as a channel disconnect on
  `recv`, a catchable `Err`, and the peer aborts safely). One is `StopOnSend`: when the party a
  message is being **sent to** has crashed, mpstthree's affine `send` performs its linear
  `cancel` and then **panics**, so the survivor crash-stops by unwinding rather than by a
  recoverable `Err`. This is still crash-stop (safe termination, no deadlock, no wrong message),
  and it is honestly **not** graceful recovery. See GAPS.md finding **F-T1**.
- **Lean/TLA+ (crash-recovery).** A crashed replica reverts to a durable checkpoint and
  **continues**; the theorems are about the **budget bound** surviving this, under a discipline
  (write-ahead debit, durable dedup) whose necessity is machine-checked. MPST models none of the
  revert-and-continue behaviour.

So on crash the methods are complementary again: MPST covers crash-stop **communication** safety
for all interleavings; Lean/TLA+ cover crash-recovery **budget** safety (unbounded / bounded).
Neither covers the other's crash property.

## Headline

The two methods are **complementary, not redundant**. MPST gives communication-structure
guarantees (deadlock-freedom, communication safety, progress) that the Lean proof does not, and
the Lean proof gives the quantitative budget bound that MPST cannot express. Reporting this as
"a second, independent confirmation of the cap theorem" would be false: MPST never touches the
cap theorem. The value delivered is (a) new guarantees on an axis the first method left open, and
(b) a precise, tested map of the boundary between the two.

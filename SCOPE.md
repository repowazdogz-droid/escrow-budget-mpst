# SCOPE.md — exact claims, assumptions, evidence grades

Companion to GAPS.md (which is the negative space). This file states what this artifact **does**
claim and at what evidence grade.

## Claims made

1. **The transfer session is deadlock-free, communication-safe, and has progress, by
   construction.** Grade: **static / by typing** (Rust compiler + mpstthree soundness). Holds for
   all executions of the session, not a bounded set. Evidence: `src/bin/transfer_normal.rs`
   type-checks and `fork_mpst` accepts the two dual endpoints; the endpoints are the mechanical
   projections of the global type `G` (PROTOCOL.md §3).

2. **The type discipline actually enforces conformance.** Grade: **demonstrated by negative
   control.** Evidence: `examples/nc1_wrong_message.rs` (out-of-order / wrong payload) fails to
   compile with `E0599`; `examples/nc2_not_projectable.rs` (non-dual, non-projectable) fails to
   compile with `E0631`. Driven and asserted by `scripts/negative-controls.sh`. If either
   compiled, that would be a finding; neither does.

3. **A crash at every protocol point leaves the surviving peer deadlock-free and reception-error-
   free, under crash-stop.** Grade: **structural for the guarantee, executed for the witnesses.**
   The guarantee is a property of the affine session typing (safe cancellation on disconnect);
   the specific crash points are exercised in `src/bin/transfer_crash_stop.rs` (six scenarios,
   captured in `runs/transfer_crash_stop.out.txt`). One scenario is `StopOnSend` (safe abort by
   panic-unwind, not graceful recovery); see GAPS.md F-T1.

## Claims explicitly NOT made (pointers to GAPS.md)

- **Not** the budget bound `Σspent ≤ CAP` (N/A to MPST; GAPS.md §1).
- **Not** reorder/duplicate/loss tolerance (assumed-away reliable channels; GAPS.md §1).
- **Not** crash-recovery (only crash-stop; GAPS.md §2).
- **Not** any Byzantine-fault property (GAPS.md §3).
- **Not** a refinement between this Rust model and the Lean/TLA+ models (GAPS.md §4).
- **Not** an independent re-proof of the sibling's cap theorem. The methods are complementary
  (COMPARISON.md).

## Assumptions

- **Reliable, ordered, in-process channels** (mpstthree over crossbeam). This is stronger than the
  escrow protocol's real transport and is the key modelling assumption limiting reach.
- **Static two-role sessions.** One transfer = one binary session `{S, R}`. The protocol has no
  genuine multiparty synchronisation (PROTOCOL.md F-M4).
- **Non-Byzantine participants.** Same assumption the sibling makes.
- **mpstthree encoding soundness** (Lagaillardie, Neykova & Yoshida): a well-typed program conforms
  to its session type. Trusted, not re-established here.
- **The `Credit/Refuse` acknowledgement is a modelling addition** over the fire-and-forget protocol
  (PROTOCOL.md F-M1).

## Evidence grades used (kept distinct, never upgraded)

| Grade | Meaning | Where |
|---|---|---|
| **static / by typing** | Guaranteed for all executions by the Rust type system + mpstthree soundness | deadlock-freedom, communication safety, progress, conformance |
| **negative control** | A deliberately broken variant is rejected at compile time | nc1, nc2 |
| **executed witness** | A specific run illustrating a typed guarantee or exercising a crash path | transfer_normal, transfer_crash_stop runs |
| **source-read** | A property of the tooling confirmed by reading the crate source, not by an audit | F-T1 (affine send panics on disconnect) |

## Trusted computing base

See GAPS.md §4. In short: the Rust compiler and type system, mpstthree 0.1.17 and its macros,
`mpst-seq-proc` 0.1.14, `either`, `crossbeam-channel`, and the executed binaries. Larger than the
Lean kernel TCB of the sibling; this is the price of the method.

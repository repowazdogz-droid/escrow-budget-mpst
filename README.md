# escrow-budget-mpst

A **second, independent verification method** applied to the existing
[`escrow-budget`](../escrow-budget/) artifact: the transfer sub-protocol expressed as a
**multiparty session type (MPST) with crash-stop failures**, implemented in **Rust** so the type
system enforces protocol conformance statically.

This is **complementary, not confirmatory**. The first method (Lean 4 + TLA+, in the sibling
directory) proves the quantitative budget bound `Σspent ≤ CAP`. This method establishes
communication-structure properties (deadlock-freedom, communication safety, progress) and
crash-stop behaviour that the first method explicitly did **not** claim. Neither re-proves the
other. The point of the exercise is two independent techniques plus an exact account of where
they do **not** overlap. See [`COMPARISON.md`](COMPARISON.md).

## The one-paragraph result

MPST gives, by construction (checked by the Rust compiler on every build), that the transfer
session is deadlock-free and communication-safe on all executions, and that a crash at any
protocol point leaves the surviving peer deadlock-free (crash-stop, safe abort). MPST **cannot**
express the budget bound at all: session types govern communication, not arithmetic state, and the
`Charge` action that actually consumes budget sends no message, so it is invisible to the type.
Conversely, the Lean proof gives the budget bound for all N but no liveness or progress. The two
methods are therefore complementary. Claiming this as "independent confirmation of the cap
theorem" would be false, because this method never touches the cap theorem.

## What is here

| Deliverable | File |
|---|---|
| Protocol in prose, checked against the sibling TLA+, + encoding-vs-TLA+ findings | [`PROTOCOL.md`](PROTOCOL.md) |
| Global type, projections (abstract) | [`PROTOCOL.md`](PROTOCOL.md) §2–§3 |
| Global type + projections (as Rust types) and normal-operation run | `src/bin/transfer_normal.rs` |
| Crash-stop fault injection at every protocol point | `src/bin/transfer_crash_stop.rs` |
| Negative control 1: type-violating impl rejected at compile time | `examples/nc1_wrong_message.rs` |
| Negative control 2: non-projectable global type rejected at compile time | `examples/nc2_not_projectable.rs` |
| Meta-control: an unrelated compile error is NOT counted as a rejection | `examples/nc0_wrong_reason.rs` |
| Properties × methods table, complementary-not-redundant conclusion | [`COMPARISON.md`](COMPARISON.md) |
| What this method does NOT give; failure-model reach; TCB additions | [`GAPS.md`](GAPS.md) |
| Exact claims, assumptions, evidence grades | [`SCOPE.md`](SCOPE.md) |
| Captured run outputs | `runs/` |

## Reproduce from clean

Requires a Rust toolchain (built with rustc/cargo 1.95) and network access on the first build to
fetch crates from crates.io.

```
make check            # build both bins, run both demonstrations, run the negative controls
```

`make check` will:

1. build `transfer_normal` and `transfer_crash_stop`;
2. run `transfer_normal` (normal operation: a fresh transfer credits, a duplicate is deduped/refused);
3. run `transfer_crash_stop` (a crash injected at each of the six protocol points; every one
   terminates both endpoints with no deadlock and no reception error);
4. run `scripts/negative-controls.sh`, which compiles the two negative controls and asserts each
   **fails to compile with its expected error** (a compile success, or a failure for some other
   reason, is a finding). A meta-control, `nc0_wrong_reason`, fails for an unrelated reason and
   must be classed as such, which shows the error check can tell reasons apart. The real compiler
   errors are saved to `negative/*.stderr.txt`.

Full rebuild from an empty target directory:

```
cargo clean && make check
```

Run pieces individually:

```
cargo run --bin transfer_normal
cargo run --bin transfer_crash_stop
bash scripts/negative-controls.sh
```

## Note on the tooling

We use an existing MPST-for-Rust implementation, **mpstthree 0.1.17**; we do not invent a session-
type encoding. Its generated macros reference three crates by bare path, so a downstream crate
must list them as direct dependencies (`mpst-seq-proc`, `either`, `crossbeam-channel`); this is
recorded in `Cargo.toml`. Crash-stop uses mpstthree's **affine-timed** variant. mpstthree also
ships a separate top-down kMC well-formedness checker (`checker_concat!`), which we did **not**
run because its k-checking step requires an external Haskell `KMC` binary that is not installed
here; well-formedness in this artifact is the compile-time channel-duality check instead, which
needs no external tool. See GAPS.md §5.

## Citing the method (no novelty is claimed for MPST itself)

- K. Honda, N. Yoshida, M. Carbone. *Multiparty Asynchronous Session Types.* POPL 2008; journal
  version JACM 2016. (Global types, projection, deadlock-freedom / communication safety by typing.)
- N. Lagaillardie, R. Neykova, N. Yoshida. *Implementing Multiparty Session Types in Rust.*
  COORDINATION 2020. (The mpstthree approach: `MeshedChannels`, duality-checked projections.)
- N. Lagaillardie, R. Neykova, N. Yoshida. *Stay Safe under Panic: Affine Rust Programming with
  Multiparty Session Types.* ECOOP 2022. (The affine variant used here for crash-stop / safe
  cancellation.)
- A. D. Barwell, A. Scalas, N. Yoshida, F. Zhou. *Generalised Multiparty Session Types with
  Crash-Stop Failures.* CONCUR 2022. (The crash-stop failure model this artifact compares against.)
- J. Lange, N. Yoshida. *Verifying Asynchronous Interactions via Communicating Session Automata* /
  the kMC bound. (The `checker_concat!` top-down check; not run here.)

All novelty in this repository is the **application** of these methods to the escrow-budget
protocol and the **comparison** in COMPARISON.md / GAPS.md, not the methods themselves.

## Cross-link

Sibling artifact (first method, budget bound): [`../escrow-budget/`](../escrow-budget/). Read both
to see the complementary picture; `../escrow-budget/README.md` proves what this one cannot, and
this one guarantees what that one left open.

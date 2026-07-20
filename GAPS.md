# GAPS.md — what this method does NOT give, and where its reach ends

This file is the negative space of the artifact. It is deliberately long, because the value of a
second method is only as good as the honesty about where it stops.

## 1. What MPST does not establish here

- **The budget bound `Σspent ≤ CAP`.** Not established, and not expressible. Session types govern
  the order and typing of communication, not arithmetic invariants over aggregate state. The
  `Charge` action that consumes budget sends no message and is not a session event. This is the
  central non-overlap; the whole point of the first (Lean) method is exactly this bound.
- **Anything about aggregate or per-replica escrow/spent values.** No solvency (`escrow ≥ 0`), no
  conservation, no non-creation. All of these are state predicates outside the type system.
- **Message reorder / duplication / loss.** Assumed away. mpstthree uses reliable, ordered,
  in-process channels; vanilla MPST assumes reliable ordered transport. The escrow protocol's
  entire dedup/`Drop` machinery exists to survive an unreliable network, and this method does not
  model that network. So the properties that machinery defends are untouched here.

## 2. Crash-stop is not the protocol's failure model (F-T1, F-M3)

The protocol's failures are **crash-recovery**: a replica reverts to a durable checkpoint and
**continues**, under write-ahead debit and durable dedup. What this artifact models, via
mpstthree's affine-timed variant, is **crash-stop**: a participant crashes and **stops
permanently**, and the peer safely aborts.

These are different failure models. Concretely:

- Crash-stop typing has no representation of durable vs volatile state, of a checkpoint, or of
  recover-and-continue. The Lean F2 theorem (`durable_reachable_safe`) is entirely about the
  recover-and-continue discipline; none of it is inside this method.
- **F-T1 (tooling behaviour, verified against the crate source).** mpstthree's affine `recv`
  returns a catchable `Err` when the peer has crashed (clean safe-abort). Its affine `send`,
  however, calls its linear `cancel` and then **`panic!`s** on a disconnected peer
  (`binary_atmp/send.rs`, the `Err(e) => { cancel(s); panic!(...) }` arms). So a survivor that is
  **sending to** a crashed peer terminates by panic-unwind, not by a recoverable result. Our
  harness catches this (`catch_unwind`) and classifies it as `StopOnSend`. It is safe (no
  deadlock, no reception error, linear cleanup runs) but it is **not** graceful recovery, and a
  program wanting to continue past a peer crash on a send cannot do so with this API. We report
  this rather than hide it; it is a real limit of the crash handling available in the tool.

The crash-stop model **does** faithfully establish one useful thing the sibling never did: at
**every** protocol point, a crash leaves the surviving peer deadlock-free and free of reception
errors. That is a real, structural, all-interleavings guarantee about the communication skeleton.

## 3. Byzantine behaviour is out of scope

Crash-stop covers a participant that halts. It does **not** cover a participant that lies: sends a
well-typed but semantically wrong message (a `Transfer` for an amount it never debited, a
`Credit` for a `tid` it never received, a forged `dest`). Session types constrain the **type and
order** of messages, not their **truthfulness**. A Byzantine `R` that emits a well-typed `Credit`
after silently double-crediting is perfectly well-typed and would be accepted. Nothing in this
artifact, and nothing in the sibling (which also assumes non-Byzantine replicas), addresses this.
The real failure model of a distributed budget service may well include Byzantine actors; neither
method here speaks to that.

## 4. What the Rust runtime adds to the trusted base

The Lean proof's trusted base is small (the Lean kernel, `propext`, `Quot.sound`). This method's
trusted base is larger, and using it as "verification" means trusting all of it:

- **The Rust compiler and type system** as the checker of projection compatibility (duality). We
  trust that a program which type-checks really does conform to its session type. This is the
  soundness claim of the mpstthree encoding (Lagaillardie, Neykova & Yoshida), which we rely on
  and do not re-establish.
- **mpstthree (0.1.17) and its macros.** The `generate!` / `generate_atmp!` macros, the
  `MeshedChannels` runtime, the affine `cancel`, and the timed send/recv are all trusted. We read
  the affine `send`/`recv` source to characterise F-T1 but did not audit the crate.
- **`crossbeam-channel`, `either`, `mpst-seq-proc`** and the rest of the dependency graph.
- **The runtime itself is executed, not proved.** transfer_normal/transfer_crash_stop are
  **executions**, not proofs. Deadlock-freedom and communication safety come from the **types**
  (static), but the specific runs are witnesses, not exhaustive checks. The static guarantee is
  the load-bearing part; the runs illustrate it and exercise the crash paths.
- **No machine-checked refinement** connects this Rust session model to the Lean or TLA+ models.
  As with the sibling's own Lean↔TLA+↔Python relationship, the three are cross-read at the level
  of the protocol, not connected by a proof. This artifact adds a fourth model (Rust/MPST) at the
  same evidential standing: same protocol, different method, no refinement claimed.

## 5. Tooling reach: what mpstthree could and could not express for this protocol

Per the brief: if the tooling cannot express something the protocol needs, that is a result.

- **Could express:** two-party request/response with a receiver-chosen branch; the send-before-
  receive **ordering** of a transfer; compile-time rejection of out-of-order and wrong-payload
  implementations; compile-time rejection of non-projectable (non-dual) global types; crash-stop
  with safe abort on the receiving side.
- **Could not express (for this protocol):**
  - the **unreliable network** (loss / duplication / reordering). Not representable in the
    reliable-channel model; would need a different calculus (for example, session types over
    unreliable broadcast, or an explicit lossy-channel model).
  - **crash-recovery** (revert-to-durable-and-continue). The affine model gives crash-stop only.
  - the **fire-and-forget** shape without inventing an ack (F-M1). A faithful one-way session
    `S->R:Transfer.end` is expressible but carries no branch and no round-trip, so it demonstrates
    only trivial deadlock-freedom.
  - any **arithmetic / budget** content (row 1 of COMPARISON.md).
  - the separate **top-down kMC well-formedness checker** (`checker_concat!`) was **not run**: its
    k-checking step shells out to an external Haskell `KMC` binary that is not installed in this
    environment (no `cabal`/`ghc`). Well-formedness here is instead the compile-time duality check,
    which needs no external tool and is enforced on every build. This is a limit of what we ran,
    not of what MPST can express.

## 6. Net position

This method extends the escrow-budget artifact along the one axis its authors explicitly left
open (communication-structure and crash-stop liveness), and it maps precisely where it cannot
reach (budget arithmetic, the unreliable network, crash-recovery, Byzantine faults). It is a
second, independent method whose disagreements with the first are the informative part.

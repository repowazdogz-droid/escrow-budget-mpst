//! transfer_normal — the escrow *transfer* sub-protocol as a multiparty session type,
//! over RELIABLE channels (mpstthree baking / non-affine variant), normal operation.
//!
//! Roles:  S (sender / debiting replica)  and  R (receiver / crediting replica).
//! Generated names below: A = S, B = R  (mpstthree's role identifiers are A,B,...).
//!
//! Global type G  (see PROTOCOL.md §"Global type"):
//!
//!     G  =  S -> R : Transfer(u32) .
//!           R -> S : { Credit . end          // fresh tid: receiver credits once
//!                    | Refuse . end }         // duplicate tid: receiver dedups, refuses
//!
//! The receiver R chooses the branch (the dedup decision).  Projection gives:
//!
//!     G|S  =  S!R:Transfer(u32) . S?R:{Credit.end | Refuse.end}
//!     G|R  =  R?S:Transfer(u32) . R+S:{Credit.end | Refuse.end}
//!
//! WHAT THIS ESTABLISHES (by construction, statically): the two endpoints have DUAL
//! (compatible) local types, so this transfer session is communication-safe, deadlock-free
//! and has progress.  See COMPARISON.md.
//!
//! WHAT THIS DOES NOT ESTABLISH: nothing about the aggregate budget bound Σspent ≤ CAP.
//! The `charge` that consumes budget is a LOCAL action (see `Replica::charge` below) with
//! no message at all — the session type is structurally blind to it.  See COMPARISON.md /
//! GAPS.md.

#![allow(clippy::type_complexity, clippy::large_enum_variant)]

use mpstthree::binary::struct_trait::{end::End, recv::Recv, send::Send};
use mpstthree::generate;
use mpstthree::role::broadcast::RoleBroadcast;
use mpstthree::role::end::RoleEnd;
use std::error::Error;

// Create the two roles A(=S, sender) and B(=R, receiver), plus MeshedChannels, names,
// fork_mpst, offer_mpst!/choose_mpst_*! for this pair.
generate!("recursive", MeshedChannels, A, B);

// ---- Local (NON-session) replica state: the budget lives here, invisible to MPST. ----
#[derive(Clone, Copy)]
struct Replica {
    escrow: u32, // local spend rights
    spent: u32,  // consumed
}
impl Replica {
    /// Charge is a LOCAL action: no message, no session step. MPST never sees it.
    fn charge(&mut self, a: u32) -> bool {
        if a <= self.escrow {
            self.escrow -= a;
            self.spent += a;
            true
        } else {
            false // local no-overspend guard (an arithmetic fact, not a typing fact)
        }
    }
}

// ---- The session types (projections of G) ----

// Branch chosen by R(=B) and offered to S(=A). Variants wrap S's continuation (End).
enum TransferOutcome {
    Credit(MeshedChannels<End, RoleEnd, NameA>),
    Refuse(MeshedChannels<End, RoleEnd, NameA>),
}
type ChooseOutcome = Send<TransferOutcome, End>;

// S(=A): send Transfer, then receive the outcome branch.
type EndpointS = MeshedChannels<Send<u32, Recv<TransferOutcome, End>>, RoleB<RoleB<RoleEnd>>, NameA>;
// R(=B): receive Transfer, then choose+broadcast the outcome.
type EndpointR = MeshedChannels<Recv<u32, ChooseOutcome>, RoleA<RoleBroadcast>, NameB>;

// ---- Endpoint implementations (must conform to the projected types, or they don't compile) ----

fn sender(s: EndpointS, mut me: Replica, amount: u32) -> Result<(), Box<dyn Error>> {
    // Write-ahead debit BEFORE emitting (the discipline the Lean proof needs). Local, not typed.
    me.escrow = me.escrow.saturating_sub(amount);
    println!("[S] write-ahead debit done (local escrow now {}); emitting Transfer", me.escrow);
    let s = s.send(amount); // S!R : Transfer(amount)
    offer_mpst!(s, {
        TransferOutcome::Credit(s) => {
            println!("[S] transfer of {amount} was CREDITED by R");
            s.close()
        },
        TransferOutcome::Refuse(s) => {
            println!("[S] transfer of {amount} was REFUSED (duplicate) by R");
            s.close()
        },
    })
}

fn receiver(s: EndpointR, fresh_tid: bool, mut me: Replica) -> Result<(), Box<dyn Error>> {
    let (amount, s) = s.recv(); // R?S : Transfer(amount)
    if fresh_tid {
        me.escrow += amount; // credit exactly once (receiver-side dedup discipline)
        let s = choose_mpst_b_to_all!(s, TransferOutcome::Credit);
        println!("[R] credited {amount} (fresh tid). escrow now {}", me.escrow);
        s.close()
    } else {
        // duplicate tid: dedup -> refuse, do NOT credit again
        let s = choose_mpst_b_to_all!(s, TransferOutcome::Refuse);
        println!("[R] refused duplicate of {amount} (already credited)");
        s.close()
    }
}

fn run_one(fresh_tid: bool) {
    let s = Replica { escrow: 10, spent: 0 };
    let r = Replica { escrow: 0, spent: 0 };
    let (ts, tr) = fork_mpst(
        move |chan| sender(chan, s, 3),
        move |chan| receiver(chan, fresh_tid, r),
    );
    ts.join().unwrap();
    tr.join().unwrap();
}

fn main() {
    println!("== normal operation: fresh transfer (should CREDIT) ==");
    run_one(true);
    println!("\n== normal operation: duplicate transfer (should REFUSE / dedup) ==");
    run_one(false);

    // The action that actually CONSUMES budget is a LOCAL charge — no message, no session step.
    // MPST governs communication structure; it is structurally blind to this. The aggregate
    // bound Σspent ≤ CAP is the Lean proof's job, not the session type's. (See COMPARISON.md.)
    let mut a = Replica { escrow: 10, spent: 0 };
    let _ = a.charge(4);
    let _ = a.charge(4);
    let overspend = a.charge(4); // 12 > 10: rejected by a LOCAL arithmetic guard, not by any type
    println!(
        "\n[local, NON-session] after two charges: spent={} escrow={}; third charge accepted={} \
         (this arithmetic is INVISIBLE to the session type)",
        a.spent, a.escrow, overspend
    );

    println!("\nDeadlock-free & communication-safe BY CONSTRUCTION (both endpoints type-checked).");
}

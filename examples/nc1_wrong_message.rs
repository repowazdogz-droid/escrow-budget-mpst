//! NEGATIVE CONTROL 1 — a participant that VIOLATES its projected local type.
//!
//! Expectation: this file MUST FAIL TO COMPILE. If it compiles, the session-type discipline
//! is not actually enforcing protocol conformance, and THAT would be the finding.
//!
//! The sender's projected local type is  G|S = S!R:Transfer(u32) . S?R:{Credit|Refuse} . end,
//! i.e. the FIRST action must be a SEND of a u32. Below the sender instead tries to `recv`
//! first (wrong operation / out of order) AND, on the send path, sends a &str payload where a
//! u32 is required (wrong message). Either is a type error; the session type rejects it
//! statically.  Driven by scripts/negative-controls.sh, which asserts the compile fails.

#![allow(clippy::type_complexity, clippy::large_enum_variant, dead_code)]

use mpstthree::binary::struct_trait::{end::End, recv::Recv, send::Send};
use mpstthree::generate;
use mpstthree::role::broadcast::RoleBroadcast;
use mpstthree::role::end::RoleEnd;
use std::error::Error;

generate!("recursive", MeshedChannels, A, B);

enum TransferOutcome {
    Credit(MeshedChannels<End, RoleEnd, NameA>),
    Refuse(MeshedChannels<End, RoleEnd, NameA>),
}
type ChooseOutcome = Send<TransferOutcome, End>;

// Correct projected types (identical to transfer_normal.rs).
type EndpointS = MeshedChannels<Send<u32, Recv<TransferOutcome, End>>, RoleB<RoleB<RoleEnd>>, NameA>;
type EndpointR = MeshedChannels<Recv<u32, ChooseOutcome>, RoleA<RoleBroadcast>, NameB>;

fn sender(s: EndpointS) -> Result<(), Box<dyn Error>> {
    // VIOLATION (out of order): the projected type's first action is a SEND, but we call recv.
    let (_wrong, s) = s.recv();
    // VIOLATION (wrong message): even on the send path, the payload must be u32, not &str.
    let s = s.send("not-a-u32");
    offer_mpst!(s, {
        TransferOutcome::Credit(s) => s.close(),
        TransferOutcome::Refuse(s) => s.close(),
    })
}

fn receiver(s: EndpointR) -> Result<(), Box<dyn Error>> {
    let (_amount, s) = s.recv();
    let s = choose_mpst_b_to_all!(s, TransferOutcome::Credit);
    s.close()
}

fn main() {
    let (ts, tr) = fork_mpst(sender, receiver);
    ts.join().unwrap();
    tr.join().unwrap();
}

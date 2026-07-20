//! NEGATIVE CONTROL 2 — a global type that is NOT well-formed / NOT projectable to compatible
//! local types (a communication mismatch / structural deadlock).
//!
//! Expectation: this file MUST FAIL TO COMPILE. If it compiles, well-formedness is not being
//! checked, and THAT would be the finding.
//!
//! In MPST-for-Rust, projection compatibility is enforced by CHANNEL DUALITY: `fork_mpst`
//! only accepts endpoints whose paired binary session types are mutual duals (the dual of
//! `Send<T, C>` is `Recv<T, dual C>`). A non-projectable global type shows up as NON-DUAL
//! endpoint types. Here BOTH roles are given a leading `Send` (both want to speak, neither
//! will listen) — the classic un-projectable / deadlocked interaction — so their types are not
//! dual and `fork_mpst` fails to type-check. This is projection/well-formedness REJECTING an
//! ill-formed protocol, checked by rustc on every build.
//!
//! (mpstthree also ships a separate top-down kMC checker, `checker_concat!`, but its k-checking
//! step shells out to an external Haskell `KMC` binary that is not installed in this
//! environment; the duality check used here needs no external tool and is strictly compile-time.)

#![allow(clippy::type_complexity, dead_code)]

use mpstthree::binary::struct_trait::{end::End, send::Send};
use mpstthree::generate;
use mpstthree::role::end::RoleEnd;
use std::error::Error;

generate!("recursive", MeshedChannels, A, B);

// NON-DUAL: both S and R start by SENDING a u32 to each other. There is no receiver for either
// message => the global type S->R AND R->S-with-both-sending is not projectable to compatible
// (dual) locals. Dual of Send<u32,End> is Recv<u32,End>, which neither endpoint offers.
type EndpointS = MeshedChannels<Send<u32, End>, RoleB<RoleEnd>, NameA>;
type EndpointR = MeshedChannels<Send<u32, End>, RoleA<RoleEnd>, NameB>;

fn sender(s: EndpointS) -> Result<(), Box<dyn Error>> {
    let s = s.send(1u32);
    s.close()
}
fn receiver(s: EndpointR) -> Result<(), Box<dyn Error>> {
    let s = s.send(2u32);
    s.close()
}

fn main() {
    // fork_mpst requires the two endpoints' binary types to be dual; they are not -> type error.
    let (ts, tr) = fork_mpst(sender, receiver);
    ts.join().unwrap();
    tr.join().unwrap();
}

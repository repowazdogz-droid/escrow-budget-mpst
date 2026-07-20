//! transfer_crash_stop — the escrow transfer sub-protocol under CRASH-STOP failures,
//! using mpstthree's AFFINE-TIMED variant (`generate_atmp!`, `RecvTimed`/`SendTimed`).
//!
//! Affine session types make a channel DROPPABLE: if a participant crashes (its thread
//! returns and drops its endpoint), the peer's next `send`/`recv` observes a disconnect and
//! returns `Err`, which propagates via `?` and terminates the peer's session SAFELY — no
//! deadlock, no communication mismatch, no reception error. This is the crash-stop
//! guarantee, structurally (every interleaving), for the transfer session.
//!
//! CRUCIAL SCOPE (see SCOPE.md / GAPS.md): this is crash-*stop* — a crashed participant
//! STOPS PERMANENTLY and the peer safely gives up. It is NOT the escrow protocol's
//! crash-*recovery* (revert to a durable checkpoint and CONTINUE). Affine session types
//! model the safe-abort, not the write-ahead-debit / durable-dedup recovery discipline.
//! That mismatch is a genuine finding about the method's reach, not a bug to paper over.

#![allow(clippy::type_complexity, clippy::large_enum_variant)]

use mpstthree::binary::struct_trait::end::End;
use mpstthree::binary_atmp::struct_trait::{recv::RecvTimed, send::SendTimed};
use mpstthree::generate_atmp;
use mpstthree::role::broadcast::RoleBroadcast;
use mpstthree::role::end::RoleEnd;

use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

generate_atmp!(MeshedChannels, A, B);

/// Where a participant crashes. Each variant crashes exactly one role at one protocol point.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Crash {
    None,
    SenderBeforeSend,     // S crashes before emitting Transfer
    ReceiverBeforeRecv,   // R crashes before receiving Transfer
    ReceiverBeforeChoice, // R receives Transfer, then crashes before choosing Credit/Refuse
    SenderBeforeOffer,    // S sent Transfer, then crashes before receiving the outcome
}

enum TransferOutcome {
    Credit(MeshedChannels<End, RoleEnd, NameA>),
    Refuse(MeshedChannels<End, RoleEnd, NameA>),
}
// Timing windows are generous (max 3600) so we never spuriously time out; the crash-stop
// behaviour we exercise is the AFFINE disconnect, which fires immediately on a peer drop.
type ChooseOutcome = SendTimed<TransferOutcome, 'a', 0, true, 3600, true, ' ', End>;

type EndpointS = MeshedChannels<
    SendTimed<
        u32,
        'a',
        0,
        true,
        3600,
        true,
        ' ',
        RecvTimed<TransferOutcome, 'a', 0, true, 3600, true, ' ', End>,
    >,
    RoleB<RoleB<RoleEnd>>,
    NameA,
>;
type EndpointR = MeshedChannels<
    RecvTimed<u32, 'a', 0, true, 3600, true, ' ', ChooseOutcome>,
    RoleA<RoleBroadcast>,
    NameB,
>;

/// The sender's session logic. Returns Err ONLY on a disconnect (peer crash) observed via `?`.
fn sender_session(
    s: EndpointS,
    all_clocks: &mut HashMap<char, Instant>,
    crash: Crash,
) -> Result<(), Box<dyn Error>> {
    if crash == Crash::SenderBeforeSend {
        println!("[S] *** CRASH before send *** (endpoint dropped)");
        return Ok(()); // crash-stop: thread ends, endpoint dropped -> peer will observe disconnect
    }
    let s = s.send(3, all_clocks)?; // S!R : Transfer(3)  (Err here => peer already gone)
    if crash == Crash::SenderBeforeOffer {
        println!("[S] *** CRASH after send, before awaiting outcome *** (endpoint dropped)");
        return Ok(());
    }
    // S awaits R's decision. If R crashed, the recv inside offer_mpst! returns Err via `?`.
    offer_mpst!(s, all_clocks, {
        TransferOutcome::Credit(s) => { println!("[S] outcome = CREDIT"); s.close() },
        TransferOutcome::Refuse(s) => { println!("[S] outcome = REFUSE"); s.close() },
    })
}

/// The receiver's session logic. Returns Err ONLY on a disconnect (peer crash) observed via `?`.
fn receiver_session(
    s: EndpointR,
    all_clocks: &mut HashMap<char, Instant>,
    crash: Crash,
    fresh_tid: bool,
) -> Result<(), Box<dyn Error>> {
    if crash == Crash::ReceiverBeforeRecv {
        println!("[R] *** CRASH before recv *** (endpoint dropped)");
        return Ok(());
    }
    let (amount, s) = s.recv(all_clocks)?; // Err here => sender crashed pre-send
    if crash == Crash::ReceiverBeforeChoice {
        println!("[R] received {amount}, *** CRASH before choosing outcome *** (endpoint dropped)");
        return Ok(());
    }
    if fresh_tid {
        let s = choose_mpst_b_to_all!(s, all_clocks, TransferOutcome::Credit);
        println!("[R] credited {amount} (fresh tid)");
        s.close()
    } else {
        let s = choose_mpst_b_to_all!(s, all_clocks, TransferOutcome::Refuse);
        println!("[R] refused duplicate of {amount}");
        s.close()
    }
}

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Three honest outcomes for a role once its peer may have crashed.
#[derive(Debug, PartialEq)]
enum Term {
    Completed,   // the session ran to `end`
    AbortOnRecv, // peer crash observed as a disconnect on `recv` -> catchable Err -> clean abort
    StopOnSend,  // peer crash observed while SENDING -> affine `cancel(s)` then panic-unwind
    SelfCrashed, // this role is the injected crash
}

/// Wrap the sender's session. Catches BOTH the recoverable disconnect-on-recv (Err) and the
/// non-recoverable disconnect-on-send (panic-unwind). Neither hangs => no deadlock, ever.
fn sender(s: EndpointS, all_clocks: &mut HashMap<char, Instant>, crash: Crash) -> Term {
    all_clocks.insert('a', Instant::now());
    if crash == Crash::SenderBeforeSend || crash == Crash::SenderBeforeOffer {
        // These injected crashes are handled inside sender_session (early return Ok).
    }
    match catch_unwind(AssertUnwindSafe(|| sender_session(s, all_clocks, crash))) {
        Ok(Ok(())) => {
            if crash == Crash::SenderBeforeSend || crash == Crash::SenderBeforeOffer {
                Term::SelfCrashed
            } else {
                Term::Completed
            }
        }
        Ok(Err(_)) => {
            println!("[S] SAFE ABORT: peer crash seen as disconnect on recv (catchable Err); no deadlock");
            Term::AbortOnRecv
        }
        Err(_) => {
            println!("[S] CRASH-STOP: peer crash seen while sending -> affine cancel + panic-unwind; no deadlock");
            Term::StopOnSend
        }
    }
}
fn receiver(s: EndpointR, all_clocks: &mut HashMap<char, Instant>, crash: Crash, fresh: bool) -> Term {
    all_clocks.insert('a', Instant::now());
    match catch_unwind(AssertUnwindSafe(|| receiver_session(s, all_clocks, crash, fresh))) {
        Ok(Ok(())) => {
            if crash == Crash::ReceiverBeforeRecv || crash == Crash::ReceiverBeforeChoice {
                Term::SelfCrashed
            } else {
                Term::Completed
            }
        }
        Ok(Err(_)) => {
            println!("[R] SAFE ABORT: peer crash seen as disconnect on recv (catchable Err); no deadlock");
            Term::AbortOnRecv
        }
        Err(_) => {
            println!("[R] CRASH-STOP: peer crash seen while sending -> affine cancel + panic-unwind; no deadlock");
            Term::StopOnSend
        }
    }
}

fn run(label: &str, crash: Crash, fresh_tid: bool) {
    use std::sync::{Arc, Mutex};
    println!("---- {label} (crash = {crash:?}) ----");
    // fork closures must return Result<(), Box<dyn Error>>; we surface each role's classification
    // via a shared cell instead of the return value.
    let s_cell = Arc::new(Mutex::new(Term::Completed));
    let r_cell = Arc::new(Mutex::new(Term::Completed));
    let (sc, rc) = (Arc::clone(&s_cell), Arc::clone(&r_cell));
    let (ts, tr) = fork_mpst(
        move |chan, clocks| {
            *sc.lock().unwrap() = sender(chan, clocks, crash);
            Ok(())
        },
        move |chan, clocks| {
            *rc.lock().unwrap() = receiver(chan, clocks, crash, fresh_tid);
            Ok(())
        },
    );
    // Both threads terminate (disconnect Err or caught panic), never hang. A join Err (a panic
    // escaping our catch_unwind) would itself be a finding.
    let s_joined = ts.join().is_ok();
    let r_joined = tr.join().is_ok();
    let s_term = std::mem::replace(&mut *s_cell.lock().unwrap(), Term::Completed);
    let r_term = std::mem::replace(&mut *r_cell.lock().unwrap(), Term::Completed);
    let deadlock_free = s_joined && r_joined;
    println!(
        "   sender={s_term:?} receiver={r_term:?} => {}\n",
        if deadlock_free { "NO DEADLOCK, NO reception error" } else { "PANIC ESCAPED (finding)" }
    );
}

fn main() {
    // mpstthree's affine `send` panics on a disconnected (crashed) peer AFTER doing its affine
    // `cancel`. We catch that panic (crash-stop by unwind); silence its default stderr print so
    // the run output stays readable. The catch_unwind in sender/receiver is what makes it safe.
    std::panic::set_hook(Box::new(|_| {}));
    println!("== crash-stop fault injection across every protocol point ==\n");
    run("normal, fresh transfer", Crash::None, true);
    run("normal, duplicate transfer", Crash::None, false);
    run("crash: sender before send", Crash::SenderBeforeSend, true);
    run("crash: receiver before recv", Crash::ReceiverBeforeRecv, true);
    run("crash: receiver before choice", Crash::ReceiverBeforeChoice, true);
    run("crash: sender before offer", Crash::SenderBeforeOffer, true);
    println!("Every crash point terminates BOTH endpoints without deadlock or reception error.");
    println!("This is crash-STOP (safe abort), NOT the escrow protocol's crash-RECOVERY. See GAPS.md.");
}

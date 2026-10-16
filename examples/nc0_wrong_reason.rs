// Meta-control for scripts/negative-controls.sh, not a session-type check.
//
// This fails to compile for a reason unrelated to the session types (an unresolved name,
// E0425). The script runs it against nc1's expected marker and requires the verdict
// "rejected for the wrong reason". If it were counted as a session-type rejection, the
// marker check would be inert and nc1/nc2 would pass on any compile error at all.
fn main() {
    let _ = undefined_binding_for_meta_control;
}

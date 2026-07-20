# escrow-budget-mpst — second, independent verification method (MPST / crash-stop, Rust).
# See README.md. `make check` builds everything, runs both positive demonstrations, and runs
# the negative controls (which MUST fail to compile).

CARGO ?= cargo

.PHONY: check build normal crash negatives clean fmt

check: build normal crash negatives
	@echo
	@echo "=== ALL CHECKS COMPLETE ==="
	@echo "positive: transfer_normal + transfer_crash_stop ran; negatives rejected at compile time."

build:
	$(CARGO) build --bins

# Positive control 1: normal operation over reliable channels (deadlock-free by construction).
normal: build
	@echo "===== transfer_normal ====="
	./target/debug/transfer_normal

# Positive control 2: crash-stop fault injection at every protocol point (no deadlock anywhere).
crash: build
	@echo "===== transfer_crash_stop ====="
	./target/debug/transfer_crash_stop

# Negative controls: a type-violating impl and a non-projectable global type. Both MUST be
# rejected at compile time; the script asserts that and exits non-zero if either compiles.
negatives:
	@echo "===== negative controls (must FAIL to compile) ====="
	bash scripts/negative-controls.sh

clean:
	$(CARGO) clean
	rm -f negative/*.stderr.txt

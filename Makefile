.PHONY: check coverage

check:
	@printf '%s\n' '==> cargo fmt --check'
	cargo fmt --all -- --check
	@printf '%s\n' '==> cargo clippy'
	cargo clippy --all-features --all-targets --tests -- -Dwarnings
	@printf '%s\n' '==> cargo test'
	cargo test --all-features --all-targets
	@printf '%s\n' '==> cargo test --ignored e2e'
	cargo test --all-features --test e2e -- --ignored

coverage:
	cargo tarpaulin --all-features --all-targets --out Html
	wslview tarpaulin-report.html

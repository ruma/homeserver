.PHONY: build
build:
	@script/cargo build

.PHONY: test
test:
	@script/cargo test

.PHONY: doc
doc:
	@script/cargo doc

.PHONY: ci
ci:
	@script/cargo audit
	@script/cargo fmt -- --check
	@script/cargo clippy --all-targets --all-features -- -D warnings
	@script/cargo build --all -v
	@script/cargo test -v

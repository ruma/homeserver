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
	@script/cargo build -v
	@script/cargo test -v

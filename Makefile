.PHONY: build
build:
	@script/cargo build -v

.PHONY: test
test:
	@script/cargo test -v

.PHONY: ci
ci: build test

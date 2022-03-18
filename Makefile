.PHONY: build
build: clear
	cargo build

.PHONY: fmt
fmt: clear
	cargo fmt

.PHONY: clippy
clippy: clear
	cargo clippy

.PHONY: run
run: clear
	cargo run

.PHONY: clean
clean:
	cargo clean

.PHONY: recargo
recargo:
	vim Cargo.toml

.PHONY: clear
clear:
	@for (( i=0; i<100; i++ )) ; do echo "" ; done

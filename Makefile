.PHONY: all check test verify build run container package clean

all: verify

check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
	cargo test --workspace --all-features

verify:
	./scripts/verify.sh

build:
	cargo build --workspace --release

run:
	cargo run -p lambda-safe-service

container:
	docker build --tag nigenje/lambda-safe-compiler:0.1.0 .

package:
	./scripts/package.sh

clean:
	cargo clean

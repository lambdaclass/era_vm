.PHONY: clean lint test deps submodules

LLVM_PATH?=$(shell pwd)/era-compiler-tester/target-llvm/target-final/

clean:
	rm -rf ./db
	rm -rf era-compiler-tester

lint:
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

submodules:
	git submodule update --init --recursive

deps: submodules
	cargo install compiler-llvm-builder
	cd era-compiler-tester && \
	if [ ! -d "llvm" ]; then \
		zksync-llvm clone && zksync-llvm build; \
	else \
		zksync-llvm build; \
	fi

test: deps
	export LLVM_SYS_170_PREFIX=$(LLVM_PATH) && cd era-compiler-tester && cargo run --verbose --features lambda_vm --release --bin compiler-tester -- --path  tests/solidity/simple/ --target EraVM --mode "Y+M3B3 0.8.26"

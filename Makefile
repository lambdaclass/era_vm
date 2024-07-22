.PHONY: clean lint test deps

LLVM_PATH?=$(shell pwd)/era-compiler-tester/target-llvm/target-final/

clean:
	rm -rf ./db
	rm -rf era-compiler-tester

lint:
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

deps:
	git submodule update --init --recursive
	cargo install compiler-llvm-builder
	cd era-compiler-tester && zksync-llvm clone && zksync-llvm build

test:
	export LLVM_SYS_170_PREFIX=$(LLVM_PATH) && cd era-compiler-tester && cargo run --verbose --features lambda_vm --release --bin compiler-tester -- --path  tests/solidity/simple/yul_instructions/ --target EraVM --disable-deployer --mode='Y+M3B3 0.8.26'

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
	if [ ! -d $(LLVM_PATH) ]; then \
	    cd era-compiler-tester && \
		zksync-llvm clone && zksync-llvm build; \
	fi

# CI test uses LLVM from the era-compiler-llvm repository, doesn't need to build it
ci-test:
	export LLVM_SYS_170_PREFIX=$(LLVM_PATH) && cd era-compiler-tester && cargo run --verbose --features lambda_vm --release --bin compiler-tester -- --path  tests/solidity/simple/yul_instructions/ --target EraVM --mode='Y+M3B3 0.8.26'

# Local test uses LLVM from era-compiler-tester submodule, needs to build it
test: deps ci-test

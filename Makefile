.PHONY: clean lint test deps

ARTIFACTS_DIR=./program_artifacts
PROGRAMS_DIR=./programs
ZKSOLC_YUL_FLAGS=--asm --bin --yul --overwrite
ZKSOLC_ASM_FLAGS=--eravm-assembly --bin --overwrite

YUL_PROGRAMS = $(wildcard $(PROGRAMS_DIR)/*.yul)
ASM_PROGRAMS = $(wildcard $(PROGRAMS_DIR)/*.zasm)
ARTIFACTS_YUL = $(patsubst $(PROGRAMS_DIR)/%.yul, $(ARTIFACTS_DIR)/%.artifacts.yul, $(YUL_PROGRAMS))
ARTIFACTS_ASM = $(patsubst $(PROGRAMS_DIR)/%.zasm, $(ARTIFACTS_DIR)/%.artifacts.zasm, $(ASM_PROGRAMS))

compile-programs-asm: $(ARTIFACTS_ASM)
compile-programs-yul: $(ARTIFACTS_YUL)

compile-programs: compile-programs-asm compile-programs-yul

$(ARTIFACTS_DIR)/%.artifacts.yul: $(PROGRAMS_DIR)/%.yul
	zksolc $(ZKSOLC_YUL_FLAGS) $< -o $@ --debug-output-dir $@

$(ARTIFACTS_DIR)/%.artifacts.zasm: $(PROGRAMS_DIR)/%.zasm
	zksolc $(ZKSOLC_ASM_FLAGS)  $< -o $@ --debug-output-dir $@

clean:
	rm -rf $(ARTIFACTS_DIR)
	rm -rf ./db
	rm -rf era-compiler-tester

lint:
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

deps:
	git submodule update --init --recursive
	cargo install compiler-llvm-builder
	cd era-compiler-tester && zksync-llvm clone && zksync-llvm build

test:
	LLVM_SYS_170_PREFIX=$(shell pwd)/era-compiler-tester/target-llvm/target-final/ && cd era-compiler-tester && cargo run --verbose --features lambda_vm --release --bin compiler-tester -- --path  tests/solidity/simple/yul_instructions/ --target EraVM --disable-deployer --mode='Y+M3B3 0.8.26'

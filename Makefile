.PHONY: clean lint compile-programs-asm compile-programs-yul

ARTIFACTS_DIR=./program_artifacts
PROGRAMS_DIR=./programs
ZKSOLC_YUL_FLAGS=--asm --bin --yul --overwrite
ZKSOLC_ASM_FLAGS=--zkasm --bin --overwrite

YUL_PROGRAMS = $(wildcard $(PROGRAMS_DIR)/*.yul)
ASM_PROGRAMS = $(wildcard $(PROGRAMS_DIR)/*.zasm)
ARTIFACTS_YUL = $(patsubst $(PROGRAMS_DIR)/%.yul, $(ARTIFACTS_DIR)/%.artifacts.yul, $(YUL_PROGRAMS))
ARTIFACTS_ASM = $(patsubst $(PROGRAMS_DIR)/%.zasm, $(ARTIFACTS_DIR)/%.artifacts.zasm, $(ASM_PROGRAMS))

compile-programs-asm: $(ARTIFACTS_ASM)
compile-programs-yul: $(ARTIFACTS_YUL)

compile-programs: clean compile-programs-asm compile-programs-yul

$(ARTIFACTS_DIR)/%.artifacts.yul: $(PROGRAMS_DIR)/%.yul
	zksolc $(ZKSOLC_YUL_FLAGS) $< -o $@ --debug-output-dir $@

$(ARTIFACTS_DIR)/%.artifacts.zasm: $(PROGRAMS_DIR)/%.zasm
	zksolc $(ZKSOLC_ASM_FLAGS)  $< -o $@ --debug-output-dir $@

clean:
	-rm -rf $(ARTIFACTS_DIR)

lint:
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings
test: clean compile-programs
	cargo test

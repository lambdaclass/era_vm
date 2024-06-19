.PHONY: clean lint compile-programs-asm compile-programs-yul
ARTIFACTS_DIR=./program_artifacts
PROGRAMS_DIR=./programs
ZKSOLC_YUL_FLAGS=--asm --bin --yul --overwrite
ZKSOLC_ASM_FLAGS=--zkasm --bin --overwrite

YUL_PROGRAMS := $(shell find $(PROGRAMS_DIR) -type f -name "*.yul")
ASM_PROGRAMS := $(shell find $(PROGRAMS_DIR) -type f -name "*.zasm")
ARTIFACTS_YUL := $(patsubst $(PROGRAMS_DIR)/%.yul, $(ARTIFACTS_DIR)/%.artifacts.yul, $(YUL_PROGRAMS))
ARTIFACTS_ASM := $(patsubst $(PROGRAMS_DIR)/%.zasm, $(ARTIFACTS_DIR)/%.artifacts.zasm, $(ASM_PROGRAMS))

compile-programs-asm: $(ARTIFACTS_ASM)
compile-programs-yul: $(ARTIFACTS_YUL)

compile-programs: clean compile-programs-asm compile-programs-yul

$(ARTIFACTS_DIR)/%.artifacts.yul: $(PROGRAMS_DIR)/%.yul
	zksolc $(ZKSOLC_YUL_FLAGS) $< -o $(ARTIFACTS_DIR)/$(@F) --debug-output-dir $(ARTIFACTS_DIR)/$(@F)

$(ARTIFACTS_DIR)/%.artifacts.zasm: $(PROGRAMS_DIR)/%.zasm
	zksolc $(ZKSOLC_ASM_FLAGS)  $< -o $(ARTIFACTS_DIR)/$(@F) --debug-output-dir $(ARTIFACTS_DIR)/$(@F)

clean:
	-rm -rf $(ARTIFACTS_DIR)

lint:
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings
test: clean compile-programs
	cargo nextest run --workspace --all-features --no-capture

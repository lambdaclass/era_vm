.PHONY: clean lint

ARTIFACTS_DIR=./program_artifacts
PROGRAMS_DIR=./programs
ZKSOLC_FLAGS=--asm --bin --yul --overwrite

PROGRAMS = $(wildcard $(PROGRAMS_DIR)/*.yul)
ARTIFACTS = $(patsubst $(PROGRAMS_DIR)/%.yul, $(ARTIFACTS_DIR)/%.artifacts, $(PROGRAMS))

compile-programs: $(ARTIFACTS)

$(ARTIFACTS_DIR)/%.artifacts: $(PROGRAMS_DIR)/%.yul
	zksolc $(ZKSOLC_FLAGS) $< -o $@ --debug-output-dir $@

clean:
	-rm -rf $(ARTIFACTS_DIR)

lint:
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

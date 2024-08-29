.PHONY: clean lint test deps submodules bench flamegraph era-test build-bench-contracts %.sol
.SILENT: %.sol

LLVM_PATH?=$(shell pwd)/era-compiler-tester/target-llvm/target-final/
ZKSYNC_ROOT=$(shell realpath ./zksync-era)
ZKSYNC_L1_CONTRACTS=$(ZKSYNC_ROOT)/contracts/l1-contracts/artifacts
ZKSYNC_L2_CONTRACTS=$(ZKSYNC_ROOT)/contracts/l2-contracts/artifacts-zk
ZKSYNC_SYS_CONTRACTS=$(ZKSYNC_ROOT)/contracts/system-contracts/artifacts-zk
ZKSYNC_BOOTLOADER_CONTRACT=$(ZKSYNC_ROOT)/contracts/system-contracts/bootloader/build/artifacts
ZKSYNC_BENCH_TEST_DATA=$(ZKSYNC_ROOT)/etc/contracts-test-data/artifacts-zk
ZKSYNC_BENCH_CONTRACTS=$(ZKSYNC_ROOT)/core/tests/vm-benchmark/deployment_benchmarks
BENCH_SOURCES=$(shell realpath ./deployment_benchmarks_sources)


clean:
	rm -rf ./db
	rm -rf era-compiler-tester
	rm -rf $(ZKSYNC_ROOT)

lint:
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

submodules:
	git submodule update --init --recursive --depth=1

deps: submodules
	cargo install compiler-llvm-builder
	if [ ! -d $(LLVM_PATH) ]; then \
	    cd era-compiler-tester && \
		zksync-llvm clone && zksync-llvm build; \
	fi

# Local test uses LLVM from era-compiler-tester submodule, needs to build it
test: deps
	cd era-compiler-tester && cargo run --verbose --features lambda_vm --release --bin compiler-tester -- --path tests/solidity/simple --target EraVM --mode='Y+M3B3 0.8.26'

# CI test uses LLVM from the era-compiler-llvm repository, doesn't need to build it
ci-test:
	export LLVM_SYS_170_PREFIX=$(LLVM_PATH) && $(MAKE) test

# Build the given set of zksync era contracts,
# this can be: l1, l2 or system contracts.
# These are needed for benchmarking.
define build_zk_contracts
	cd $(ZKSYNC_ROOT)/contracts && \
	yarn install --frozen-lockfile && \
	$(1)
endef

$(ZKSYNC_L1_CONTRACTS):
	$(call build_zk_contracts, yarn l1 build)

$(ZKSYNC_L2_CONTRACTS):
	$(call build_zk_contracts, yarn l2 build)

$(ZKSYNC_SYS_CONTRACTS):
	$(call build_zk_contracts, yarn sc build:system-contracts)

$(ZKSYNC_BOOTLOADER_CONTRACT):
	$(call build_zk_contracts, yarn sc build:bootloader)

$(ZKSYNC_BENCH_TEST_DATA):
	touch $(ZKSYNC_ROOT)/etc/contracts-test-data
	cd $(ZKSYNC_ROOT)/etc/contracts-test-data && yarn install --frozen-lockfile && yarn build

# Steps:
# 1 - cd
# 2 - Take the given CONTRACT.sol and get its byte code
# 3 - Parse the output
# 4 - Redirect the hexstring to a CONTRACT (mind the extension-less name) to
#     a file insie the contract benchmarks folder.
%.sol:
	echo "Building benchmark contract: $@"
	cd $(BENCH_SOURCES) && \
	zksolc --bin $@ | grep -oE '0x[0-9a-fA-F]+' > $(ZKSYNC_BENCH_CONTRACTS)/$(basename $@)

build_bench_contracts: fibonacci_rec.sol send.sol


# Compile contracts and fetch submodules for the benches.
# If you get any 'missing file' errors whil running cargo bench, this is probably what you must run.
bench-setup: submodules build_bench_contracts $(ZKSYNC_BENCH_TEST_DATA) $(ZKSYNC_SYS_CONTRACTS) $(ZKSYNC_BOOTLOADER_CONTRACT) $(ZKSYNC_L1_CONTRACTS) $(ZKSYNC_L2_CONTRACTS)

bench:
	cd $(ZKSYNC_ROOT) && cargo bench --bench criterion "$(lambda|fast_vm|legacy)/(fibonacci|send)"

check-flamegraph:
	cd $(ZKSYNC_ROOT) && CARGO_PROFILE_BENCH_DEBUG=2 cargo flamegraph --root --bench criterion -- --bench --profile-time 5 "$lambda/fibonacci_rec^"

bench-base:
	cd $(ZKSYNC_ROOT) && cargo bench --bench criterion -- --save-baseline bench_base lambda 1>bench-compare.txt

bench-compare:
	cd $(ZKSYNC_ROOT) && cargo bench --bench criterion -- --baseline bench_base lambda 1>bench-compare.txt

clean-contracts:
	rm -rfv $(ZKSYNC_BENCH_TEST_DATA) $(ZKSYNC_SYS_CONTRACTS) $(ZKSYNC_BOOTLOADER_CONTRACT) $(ZKSYNC_L1_CONTRACTS) $(ZKSYNC_L2_CONTRACTS)

era-test: submodules
	cd ./zksync-era/core/lib/multivm && cargo t era_vm

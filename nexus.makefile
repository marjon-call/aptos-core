.PHONY: build

build:
	@if [ -f nexus_build_result.yaml ]; then \
		echo "nexus_build_result.yaml already exists, skipping build."; \
	else \
		echo "Installing build dependencies..." && \
		(apt-get update -qq && apt-get install -y -qq clang lld pkg-config libssl-dev libudev-dev libdw-dev libclang-dev cmake libcurl4-openssl-dev > /dev/null 2>&1 || true) && \
		echo "Building aptos-core..." && \
		cargo check -p aptos-node && \
		echo "Build succeeded, writing nexus_build_result.yaml..." && \
		printf 'language: rust\nbuild_targets:\n  - .\ninstallation_script: "apt-get update -qq && apt-get install -y -qq clang lld pkg-config libssl-dev libudev-dev libdw-dev libclang-dev cmake libcurl4-openssl-dev && cargo check -p aptos-node"\nrun_test_command: "cargo test -p aptos-node --lib -- poc::test_poc --nocapture"\ndeveloper_note: "LFS files skipped (not needed for audit). Set git config filter.lfs.smudge=cat and filter.lfs.clean=cat if cloning fresh."\nblocking_error: ""\n' > nexus_build_result.yaml; \
	fi

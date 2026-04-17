.PHONY: build

build:
	@if [ -f nexus_build_result.yaml ]; then \
		echo "nexus_build_result.yaml already exists, skipping build."; \
	else \
		echo "Installing build dependencies..." && \
		(which clang > /dev/null 2>&1 && which pkg-config > /dev/null 2>&1 || (apt-get update -qq && apt-get install -y -qq clang lld pkg-config libssl-dev > /dev/null 2>&1)) && \
		echo "Building aptos-core..." && \
		cargo check -p aptos-node && \
		echo "Build succeeded, writing nexus_build_result.yaml..." && \
		printf 'language: rust\nbuild_targets:\n  - .\ninstallation_script: "which clang && which pkg-config || (apt-get update -qq && apt-get install -y -qq clang lld pkg-config libssl-dev) && cargo check -p aptos-node"\nrun_test_command: "cargo test -p aptos-node --lib -- poc::test_poc --nocapture"\ndeveloper_note: "LFS files skipped (not needed for audit). Set git config filter.lfs.smudge=cat and filter.lfs.clean=cat if cloning fresh. Requires clang, lld, pkg-config, libssl-dev."\nblocking_error: ""\n' > nexus_build_result.yaml; \
	fi

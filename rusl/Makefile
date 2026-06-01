# Makefile — rusl workspace 构建与测试
#
# 测试需 -Z panic-abort-tests (no_std 不兼容 unwinding)

.PHONY: build build-rel test test-c check clean lib-test inte-test

build:
	cargo build --workspace

build-rel:
	cargo build --workspace --release -Zbuild-std=core

check:
	cargo check --workspace
	cargo check --workspace --no-default-features

test:
	RUSTFLAGS="-C panic=abort" cargo +nightly -Z panic-abort-tests test --workspace -- test-threads=1

inte-test:
	RUSTFLAGS="-C panic=abort" cargo +nightly -Z panic-abort-tests test -p rusl --test integration -- test-threads=1

lib-test:
	RUSTFLAGS="-C panic=abort" cargo +nightly -Z panic-abort-tests test --workspace --lib -- test-threads=1

test-c:
	RUSTFLAGS="-C panic=abort" cargo +nightly -Z panic-abort-tests test -p rusl --no-default-features -- test-threads=1

# 单独测试某个模块的集成测试 (用法: make test-string)
test-%:
	RUSTFLAGS="-C panic=abort" cargo +nightly -Z panic-abort-tests test -p rusl --test $*_integration -- test-threads=1

clean:
	cargo clean
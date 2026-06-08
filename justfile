_default:
  cargo := "source ~/.cargo/env && cargo"

# Build
build:
  {{cargo}} build
build-release:
  {{cargo}} build --release

# Test
test:
  {{cargo}} test
test-nocapture:
  {{cargo}} test -- --nocapture

# Lint
fmt:
  {{cargo}} fmt
fmt-check:
  {{cargo}} fmt -- --check
lint:
  {{cargo}} clippy --all-targets --all-features -- -D warnings
check:
  {{cargo}} check --all-targets --all-features

# Bench
bench:
  {{cargo}} bench

# Clean
clean:
  {{cargo}} clean

# All-in-one CI preflight
ci:
  just fmt-check
  just lint
  just test
  just bench-compile

bench-compile:
  {{cargo}} bench --no-run

# Watch mode
watch:
  {{cargo}} watch -x check -x test

# Docs
doc:
  {{cargo}} doc --open

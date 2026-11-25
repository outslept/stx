# stx task runner.
#
# Every recipe here mirrors .github/workflows/ci.yml. If you change one,
# change the other — the point of this file is that `just check` and CI
# mean the same thing.

set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# --- Rust: format, lint, test ------------------------------------------------

# Rewrite formatting across the whole workspace.
fmt:
    cargo fmt --all

# Exactly what CI checks today (note: `src/` is not formatted yet, only `napi/`).
fmt-check:
    cargo fmt --check -p stx-napi

# Warnings off the table locally; CI does not pass -D warnings yet.
clippy:
    cargo clippy --locked --workspace --all-targets --all-features

clippy-strict:
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --locked --workspace --all-targets --all-features

# The CI `rust` job, runnable locally.
check: fmt-check
    cargo clippy --locked --workspace --all-targets --all-features
    cargo test --locked --workspace --all-targets --all-features

# --- Build and run -----------------------------------------------------------

build:
    cargo build --locked --release --bin stx

# just run "src/**/*.css" --fix
run *args:
    cargo run --locked --quiet --bin stx -- {{args}}

# The differential harness: reads JSON lines on stdin, one response per line.
audit-build:
    cargo build --locked --bin stx-audit

# --- Node-API package --------------------------------------------------------

napi-install:
    cd napi && npm ci --ignore-scripts --no-audit --no-fund

napi-build:
    cd napi && npm run build

napi-test:
    cd napi && npm test

# Node-API tests never build without the .node binary, so chain the install.
napi: napi-install napi-build napi-test

# The example project uses pnpm.
example:
    pnpm --dir example install
    pnpm --dir example run lint

# --- Release hygiene ---------------------------------------------------------

# The same check the CI `rust` job runs before anything else.
check-versions:
    #!/usr/bin/env bash
    npm_version=$(jq -er .version napi/package.json)
    cargo_version=$(cargo metadata --locked --no-deps --format-version 1 \
        | jq -er '.packages[] | select(.name == "stx") | .version')
    if [[ "$npm_version" != "$cargo_version" ]]; then
        printf 'npm version %s does not match Cargo version %s\n' \
            "$npm_version" "$cargo_version" >&2
        exit 1
    fi
    printf 'versions agree: %s\n' "$npm_version"

# Advisory, license, ban and source checks. Needs `cargo install cargo-deny`.
deny:
    cargo deny check

clean:
    cargo clean
    rm -f napi/*.node napi/index.js napi/index.d.ts

alias b := build
alias c := check
alias t := test

# Contributing to governance-contract

Thank you for helping with the Soroban governance contract. This is an early, unaudited project; small, well-tested changes are the easiest to review.

## Set up

- Rust (stable) with the `wasm32v1-none` target: `rustup target add wasm32v1-none`.
- The Stellar CLI 30.x (`stellar --version`).

Then, in this folder:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

That is the whole check. A pull request is ready when it passes.

## Rules for this package

- `#![no_std]`; no `unwrap`, `expect` or `panic!` in contract code. Return a documented `Error` variant.
- Every new error variant gets a doc comment; error numbers never change.
- Extend the TTL on every persistent write and emit an event on every state change.
- Every collection is bounded: one storage entry per item, fixed maximums.
- Every public function has a test for success and at least one for failure.
- Changing a public function or event also means updating `docs/CONTRACT.md` and saying so in the PR.
- Testnet only. Never put secret keys, seed phrases or tokens in code, tests, logs or issues.
- Documentation describes what exists and is tested; no aspirational claims.

## Commits and pull requests

- Conventional Commits: `feat: ...`, `fix: ...`, `docs: ...`, `test: ...`, `chore: ...`.
- One topic per pull request. Fill in the pull request template, including how you tested.
- Code marked `WAVE-ISSUE(<id>)` is where a planned issue plugs in; keep the marker until that issue is done.

## Claiming an issue

Comment on the issue that you would like to work on it and wait for a maintainer to assign it before starting. If you stop, say so, so someone else can take it. Issues labelled with a complexity (Trivial, Medium, High) describe their acceptance criteria; meet all of them.

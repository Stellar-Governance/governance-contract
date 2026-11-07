# governance-contract

Soroban smart contract for Stellar Governance: token holders deposit one SEP-41 token to get checkpointed voting power, create proposals, vote For / Against / Abstain, and execute passed proposals after a timelock. No admin key.

**Status:** skeleton only. The contract interface is built in Phase 1 (see the project `PLAYBOOK.md`).

**Audit status:** this contract has not been audited. Do not use it to hold value.

## Verify

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

Needs Rust with the `wasm32v1-none` target and the Stellar CLI (30.x).

## License

MIT

# governance-contract

Soroban smart contract for Stellar Governance. One deployment governs with one SEP-41 token:

- holders `deposit` the token to get voting power, checkpointed per ledger, and can `withdraw` at any time;
- anyone with enough power `propose`s a title, the sha256 of the full text, a link to it, and up to 5 actions (`UpdateConfig` or `Invoke` another contract);
- holders `cast_vote` For / Against / Abstain with their power at the proposal's creation ledger, so tokens moved afterwards cannot vote again;
- a proposal that reaches quorum with more For than Against is `queue`d, waits out the timelock and is `execute`d by anyone;
- there is no admin key: settings change only through a passed proposal.

**Audit status: this contract has not been audited.** Do not use it to hold value.

Full reference (functions, errors, events, storage, TTL, limits, what is not guaranteed): [docs/CONTRACT.md](docs/CONTRACT.md).

## Verify

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

Needs Rust with the `wasm32v1-none` target and the Stellar CLI (30.x). `cargo test` runs 58 tests, including a property-style invariant test that takes about 30 seconds.

Built with `soroban-sdk` 29.0.1. The optimized wasm is written to `target/wasm32v1-none/release/governance_contract.wasm`.

## Status

Phase 1 of the project plan: implemented and tested locally. Testnet deployment is next; see the repository's `docs/TESTNET.md` once it exists.

## License

MIT

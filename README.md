# governance-contract

[![ci](https://github.com/Stellar-Governance/governance-contract/actions/workflows/ci.yml/badge.svg)](https://github.com/Stellar-Governance/governance-contract/actions/workflows/ci.yml)

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

## Deploy (testnet only)

```bash
GOV_TOKEN_ID=<token contract id> GOV_DEPLOYER=<stellar CLI identity name> scripts/deploy-testnet.sh
```

The script builds the wasm, deploys it with the rehearsal config (override with the `GOV_*` variables listed at its top) and prints the new contract id. It refuses any network other than `testnet`. Signing uses a Stellar CLI identity by name; no secret is passed on the command line.

## Rehearsal scripts (testnet only)

| Script | What |
|---|---|
| `scripts/redeploy-testnet.sh` | Rebuilds everything after a testnet reset: identities (in the CLI key store), the GOV asset and balances, the contract, and the env files of the toolkit, frontend and analytics |
| `scripts/fund-treasury.sh` | Sends a token (XLM by default) to the contract |
| `scripts/propose-invoke.sh` | Creates a treasury proposal with one `Invoke` transfer |
| `scripts/e2e-testnet.md` | The full demo runbook |

A redeploy drill on 2026-11-09 took 333 seconds from the script start to written env files (most of it friendbot funding six accounts); the frontend and analytics then need a rebuild to pick up the new contract id.

## Status

Implemented and tested locally (58 tests). Deployed to testnet on 2026-11-07 as `CAOERDLG52UU6X5JENOXNKN4DWWJPVPNK7AD4RYXE3MA4TRBY62O5I5F` (wasm sha256 `b08f4266…30ef`) and taken through one full lifecycle with the CLI, then through a full demo (settings change, treasury transfer by `Invoke`, a proposal that missed the quorum, a cancellation, and a re-deposit after a snapshot that added no power). Testnet resets wipe deployments; the next scheduled reset is 2026-12-16.

Measured on testnet: rent is most of the cost of a call, because every new entry is extended to the maximum TTL (about 180 days). A first deposit costs about 0.2 XLM, a proposal about 0.2 XLM, a vote about 0.05 XLM, and `queue`/`execute` under 0.001 XLM. The deploy itself paid about 39 XLM of rent for the contract code and instance.

## Related repositories

Stellar Governance is split into four repositories. Dependence runs one way: contract, then toolkit, then frontend, then analytics.

- **governance-contract** (this repository): Soroban contract (Rust); also holds the project documents
- [governance-toolkit](https://github.com/Stellar-Governance/governance-toolkit): Node service: indexer, drafts, discussion, sign-in, read API
- [governance-frontend](https://github.com/Stellar-Governance/governance-frontend): Next.js voting dashboard
- [voting-analytics](https://github.com/Stellar-Governance/voting-analytics): static analytics site

Project documents, kept in this repository: [docs/TESTNET.md](docs/TESTNET.md) (identities, contract ids, deployment, measured fees), [docs/DEMO.md](docs/DEMO.md) (what was run on testnet and what was observed), [docs/GOVERNANCE_RISKS.md](docs/GOVERNANCE_RISKS.md) (known risks, stated plainly).

## License

MIT

# Governance contract reference

**Audit status: not audited.** Everything below describes the code in `src/` and what its tests check. Nothing here has been reviewed by a third party.

One deployment governs with one SEP-41 token. Holders deposit the token to get voting power. Anyone with enough power creates a proposal with up to 5 actions. Holders vote For / Against / Abstain with their power at the proposal's snapshot ledger. A passed proposal is queued, waits out a timelock and is then executed by anyone. There is no admin key: settings change only through a passed proposal.

Units: times are ledger timestamps in seconds (`u64`). Snapshots are ledger sequence numbers (`u32`). Amounts and power are `i128` in the token's smallest unit.

## Functions

| Function | Auth | What it does | Errors |
|---|---|---|---|
| `__constructor(token, config)` | deployer (the deploy tx) | Binds the token, stores the config. Runs once. Stores no admin. | `InvalidConfig` (deployment fails) |
| `deposit(from, amount) -> i128` | `from` | Moves `amount` from `from` into the contract; raises `from`'s deposit and the total. Returns the new deposit. | `InvalidAmount`, `TokenFailed`, `Overflow` |
| `withdraw(from, amount) -> i128` | `from` | Returns `amount` to `from`. Allowed at any time; lowers power only for snapshots after this ledger; votes already cast stay counted. | `InvalidAmount`, `InsufficientDeposit`, `TokenFailed` |
| `deposit_of(account) -> i128` | none | Current deposit (0 if none). | none |
| `power_at(account, ledger) -> i128` | none | Power from all changes in ledgers strictly before `ledger`. | `SnapshotNotPast` if `ledger` > current |
| `total_power_at(ledger) -> i128` | none | Total deposited power, same rule. | `SnapshotNotPast` |
| `propose(proposer, title, description_hash, uri, actions) -> u64` | `proposer` | Creates a proposal; ids start at 1 and are never reused. | `BelowThreshold`, `TextTooLong`, `TooManyActions`, `ForbiddenTarget`, `InvalidConfig`, `Overflow` |
| `cast_vote(voter, id, choice) -> i128` | `voter` | Counts the voter's power at the snapshot. One vote per voter, never changed. Returns the weight. | `ProposalNotFound`, `VotingClosed`, `AlreadyVoted`, `NoVotingPower`, `Overflow` |
| `cancel(caller, id)` | `caller` | Proposer only, while Pending, Active, Succeeded or Queued. | `ProposalNotFound`, `NotAuthorized`, `WrongState` |
| `queue(id) -> u64` | none (permissionless) | Succeeded -> Queued, `eta = now + timelock`. Returns the eta. | `ProposalNotFound`, `WrongState`, `Overflow` |
| `execute(id)` | none (permissionless) | Queued and `eta <= now < eta + grace`: marks executed, runs actions in order. | `ProposalNotFound`, `WrongState`, `TimelockActive`, `ActionFailed` |
| `state(id) -> ProposalState` | none | Derived from the ledger time and the proposal's flags. | `ProposalNotFound` |
| `get_proposal(id) -> Proposal` | none | The stored proposal. | `ProposalNotFound` |
| `get_vote(id, voter) -> Option<VoteRecord>` | none | How `voter` voted, if at all. | none |
| `get_config() -> Config` | none | Current settings. | none |
| `token() -> Address` | none | The governance token. | none |
| `proposal_count() -> u64` | none | Number of proposals created. | none |

### `propose` rules
- `snapshot_ledger` = the current ledger. The proposer's power is read with `power_at(proposer, snapshot_ledger)`, so a deposit made in the same ledger does not count.
- Power must be `>= proposal_threshold` (threshold 0 lets anyone propose).
- `title` <= 120 bytes, `uri` <= 200 bytes, at most 5 actions, at most 8 arguments per `Invoke` (more is `TooManyActions`).
- `Action::UpdateConfig(config)` is validated now and again at execution.
- `Action::Invoke` whose target is the governance token or this contract is `ForbiddenTarget`, so no proposal can move the deposits.
- Fixed at creation: `vote_start = now + voting_delay`, `vote_end = vote_start + voting_period`, `total_power = total_power_at(snapshot)`, `quorum`, and copies of `timelock_secs` and `grace_period_secs` (decision D19). A later config change does not alter existing proposals.

### Passing rule
Succeeded means voting has ended, `for > against`, and `for + abstain >= quorum`. A tie is Defeated. `quorum = ceil(total_power * quorum_bps / 10_000)`, at least 1 when `total_power > 0`. A proposal with `total_power == 0` can never pass.

### Execution
`execute` sets `executed = true` first, then runs the actions in order:
- `UpdateConfig` is re-validated and replaces the config. It affects only proposals created afterwards.
- `Invoke` calls `env.try_invoke_contract(contract, function, args)`. The governance contract is the direct invoker, so a target that calls `require_auth()` on the governance address (for example `token.transfer(gov, to, amount)`) is authorized without a signature (tested: `invoke_transfers_a_treasury_token`).
- If any action fails, `execute` returns `ActionFailed` and the whole transaction reverts, including the `executed` flag: the proposal stays Queued and can be retried until its grace period ends (tested: `a_failing_invoke_reverts_everything`).
- A target that calls back into the governance contract during `execute` makes the action fail (tested: `reentry_from_a_target_fails_the_action`).

## Config and bounds

| Field | Bounds |
|---|---|
| `voting_delay_secs` | 0 to 30 days |
| `voting_period_secs` | 60 s to 30 days |
| `timelock_secs` | 0 to 30 days |
| `grace_period_secs` | 60 s to 30 days |
| `proposal_threshold` | >= 0 |
| `quorum_bps` | 1 to 10,000 |

These bounds suit a testnet rehearsal. They are not a recommendation for real governance.

## State machine

```
Pending --(now >= vote_start)--> Active --(now >= vote_end, passed)--> Succeeded --queue--> Queued --execute, eta <= now--> Executed
                                   \--(now >= vote_end, not passed)--> Defeated              \--(now >= eta + grace)--> Expired
Pending, Active, Succeeded, Queued --cancel by proposer--> Canceled
```

| From | To | Trigger |
|---|---|---|
| Pending | Active | time reaches `vote_start` |
| Active | Succeeded / Defeated | time reaches `vote_end`; passing rule below |
| Succeeded | Queued | `queue` |
| Queued | Executed | `execute` with `eta <= now < eta + grace` |
| Queued | Expired | time reaches `eta + grace` without execution |
| Pending, Active, Succeeded, Queued | Canceled | `cancel` by the proposer |

Final states: Defeated, Executed, Canceled, Expired. Nothing leaves a final state. State is computed on every read; nothing is stored except the `eta`, `executed` and `canceled` fields.

## Errors

| Code | Name | Meaning |
|---|---|---|
| 1 | `InvalidConfig` | A config value is outside its allowed bounds |
| 2 | `InvalidAmount` | The amount must be greater than zero |
| 3 | `InsufficientDeposit` | The withdrawal is larger than the deposit |
| 4 | `BelowThreshold` | Proposer's power at the snapshot is below the threshold |
| 5 | `TooManyActions` | More than 5 actions, or an Invoke with more than 8 arguments |
| 6 | `ForbiddenTarget` | Invoke targets the governance token or this contract |
| 7 | `TextTooLong` | Title over 120 bytes or URI over 200 bytes |
| 8 | `ProposalNotFound` | No proposal with this id |
| 9 | `VotingClosed` | Voting is not open (including canceled proposals) |
| 10 | `AlreadyVoted` | This address already voted on this proposal |
| 11 | `NoVotingPower` | No power at the proposal's snapshot |
| 12 | `WrongState` | The proposal's state does not allow this call |
| 13 | `TimelockActive` | Queued, but `eta` has not been reached |
| 14 | `NotAuthorized` | Only the proposer may cancel |
| 15 | `ActionFailed` | An action failed during execution; nothing changed |
| 16 | `TokenFailed` | The token transfer failed; the token's own error code is not passed through, because it could collide with these codes |
| 17 | `Overflow` | An arithmetic result was out of range |
| 18 | `SnapshotNotPast` | The requested ledger is in the future |

A missing authorization is reported by the host, not with one of these codes.

## Events

The first topic is the event name. The other listed topics follow it; every remaining field is in the data map.

| Event | Topics | Data |
|---|---|---|
| `deposited` | `account` | `amount`, `deposit` (after), `total` (after) |
| `withdrawn` | `account` | `amount`, `deposit` (after), `total` (after) |
| `proposal_created` | `proposal_id`, `proposer` | `snapshot_ledger`, `vote_start`, `vote_end`, `description_hash`, `total_power`, `quorum` |
| `vote_cast` | `proposal_id`, `voter` | `choice`, `power` |
| `proposal_canceled` | `proposal_id` | none |
| `proposal_queued` | `proposal_id` | `eta` |
| `proposal_executed` | `proposal_id` | none |
| `config_updated` | `proposal_id` | `config` |

Every state change emits one of these. `Invoke` targets emit their own events too.

## Storage and TTL policy (decision D14)

| Key | Kind | Value |
|---|---|---|
| `Config`, `Token`, `ProposalCount` | instance | settings, token address, counter |
| `Proposal(id)` | persistent | one entry per proposal |
| `Vote(id, voter)` | persistent | one entry per vote |
| `Deposit(account)` | persistent | current deposit |
| `CkptCount(account)`, `Ckpt(account, i)` | persistent | one entry per checkpoint |
| `TotalCkptCount`, `TotalCkpt(i)` | persistent | one entry per total checkpoint |

No entry holds a list that grows without limit. Checkpoints are one entry each. A second change in the same ledger overwrites that ledger's checkpoint instead of adding one.

TTL: every persistent write extends the entry to the network maximum (`max_ttl`, about 180 days on testnet). Reads made by state-changing calls (`deposit`, `withdraw`, `propose`, `cast_vote`, `cancel`, `queue`, `execute`) extend an entry when fewer than 30 days remain. The instance entry is extended the same way on every state-changing call. View functions do not extend anything.

If an old checkpoint or proposal is archived anyway, nothing reads it as missing: since protocol 23, a transaction that touches an archived persistent entry restores it automatically, provided the transaction was simulated first (the RPC adds the entry to the restore list) and the caller pays the restore fee. Clients must always simulate before sending.

## Limits

| Limit | Value |
|---|---|
| Actions per proposal | 5 |
| Arguments per `Invoke` | 8 |
| Title | 120 bytes |
| URI | 200 bytes |

## What is not guaranteed

- **Not audited.**
- **No delegation.** Power belongs only to the depositor (Wave issue contract-1).
- **No veto and no guardian.** A passed proposal with a malicious `Invoke` runs after the timelock. The timelock is the only window to react, for example by withdrawing (Wave issue contract-3).
- **No upgrade path.** A bug can only be fixed by deploying a new contract and moving deposits by hand (Wave issue contract-4).
- **Linear weight only** (Wave issue contract-2).
- **Live-balance voting is not used, on purpose.** SEP-41 tokens have no historical balances, and voting with live balances lets the same tokens vote again from another address after a transfer. Power therefore comes from deposits checkpointed per ledger. Holders who do not deposit have no say.
- **Settings can lock governance.** For example, a quorum nobody can reach. There is no admin to undo it.
- `Invoke` targets are arbitrary contracts. The contract checks only that the target is not the governance token or itself. What a target does is up to voters to judge.
- Old checkpoints may need restoring (fee paid by the caller) after long inactivity; see TTL above.

## Tests

`cargo test` runs 58 tests: config bounds, deposits and checkpoints, propose and the state machine, voting, cancel, queue and execute, and a property-style test (`invariants_hold_over_random_sequences`). The property test runs three fixed-seed random sequences of 150 operations each (deposit, withdraw, token transfers between identities, propose, vote, queue, execute, cancel) and checks after every step:
- the sum of deposits equals the latest total checkpoint, which is at most the contract's token balance;
- every vote weight equals the voter's power at the snapshot, and no vote changes;
- `for + against + abstain <= total_power`, and the tallies equal the sum of vote records;
- ids are sequential and never reused; a proposal executes at most once and only from Queued; canceled, defeated and expired proposals never execute.

The targeted tests in `src/test/votes.rs` (`moved_tokens_cannot_vote_twice`) and `src/test/proposals.rs` (`invoke_on_the_token_or_this_contract_is_forbidden`) cover the double-vote and deposit-safety invariants directly.

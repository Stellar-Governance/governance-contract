# Governance risks

A short, honest list of how this governance system can go wrong. It is not a full attack analysis; that is planned as Wave issue contract-5. The contract has **not been audited**.

| Risk | What can happen | What the MVP does about it | What it does not do |
|---|---|---|---|
| Whale capture | One holder with most of the deposited power passes any proposal alone. | Nothing beyond the quorum and the timelock: power is linear in the deposit. | No delegation limits, no quadratic weight (contract-2), no veto (contract-3). |
| Low quorum | With a small quorum, a few active voters decide for everyone. | The quorum is a share of the power deposited at the snapshot, and For + Abstain must reach it. | It cannot count holders who never deposit; turnout is a share of deposited power, not of the token supply. |
| Proposal spam | Many proposals bury the important ones. | A proposal threshold (power at the snapshot). | No deposit or fee per proposal, no rate limit on-chain. |
| Timelock as the only defence | A passed malicious proposal runs after the timelock. | The timelock gives holders time to see it coming and to withdraw. | No guardian veto and no emergency stop (contract-3). |
| No upgrade path | A bug cannot be fixed in place. | Rules are fixed and known. | Fixing a bug means a new contract and moving deposits by hand (contract-4). |
| Malicious `Invoke` targets | A proposal can call any contract with any arguments as the governance contract, which can spend tokens the contract holds. | `Invoke` on the governance token or the governance contract itself is refused, so deposits can never be moved by a proposal. The frontend shows actions in words. | The contract cannot judge what a target does. Voters must read the actions. |
| Settings can lock governance | A proposal can set, for example, a quorum nobody can reach or a threshold nobody holds. | Bounds on every setting (for example quorum at most 100 %, periods at most 30 days). | No admin to undo it; a locked contract stays locked. |
| Deposit-based power | Holders must deposit to have a say; tokens in the contract are not in their wallets. | Withdrawal is always possible and does not remove votes already cast; snapshots stop the same tokens voting twice. | It does not use the token's live balances (which would allow double voting), so non-depositors are not represented. |
| State archival | Old checkpoints or proposals can be archived after long inactivity. | Entries are extended to the maximum TTL on write and when read by state-changing calls; archived entries are restored automatically when a transaction is simulated first. | Restoring costs the caller a fee; a client that skips simulation fails. |
| Off-chain text | The text people discuss lives in the toolkit, not on-chain. | The chain records the sha256 of the text; the frontend recomputes it and says whether it matches. | If the toolkit loses the text, only the hash and the link remain. |
| Event retention | The toolkit reads votes and deposits from events that RPC keeps for about 7 days. | Missing ranges are recorded as gaps and shown in the API and analytics; tallies come from contract state and stay complete. | No backfill from archives yet (toolkit-5). |
| Testnet resets | Testnet is wiped from time to time (next scheduled 2026-12-16). | Deploy and rehearsal scripts make a redeploy short. | Nothing on testnet survives a reset. |

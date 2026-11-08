# Testnet rehearsal runbook

Steps for the full demo on testnet. Identities are Stellar CLI key names; no secret is typed or printed anywhere. Every script refuses networks other than testnet.

Timings below assume the rehearsal config (`deploy-testnet.sh` defaults): voting delay 30 s, voting period 120 s, timelock 60 s, quorum 20 %, threshold 1 GOV. Waits are measured in ledger time; poll the chain or the toolkit, never the local clock.

## 0. Prerequisites

- Identities `gov-deployer`, `gov-voter-a`, `gov-voter-b`, `gov-voter-c`, `gov-treasury-recipient`, `gov-token-issuer`, funded by friendbot, with GOV trustlines and balances (see the repository's `docs/TESTNET.md` for the commands).
- The governance contract deployed: `GOV_TOKEN_ID=<GOV SAC id> scripts/deploy-testnet.sh` (saves the CLI alias `gov-governance`).
- The toolkit running against it (`governance-toolkit`: `pnpm build && pnpm start` with a `.env` for testnet).

## 1. Deposits

```bash
for v in "gov-voter-a 5000000000" "gov-voter-b 3000000000" "gov-voter-c 2000000000"; do
  set -- $v
  stellar contract invoke --id gov-governance --source-account "$1" --network testnet -- \
    deposit --from "$1" --amount "$2"
done
```

## 2. Scenario (a): config change drafted, discussed, created from the frontend's code

```bash
# In governance-toolkit: draft, publish and discuss (signs in with `stellar message sign`).
node scripts/rehearse-draft.mjs --author gov-voter-a --title "Longer voting, higher quorum" \
  --file <text.md> --comment gov-voter-b="Agreed" --comment gov-voter-c="Fine by me"
# prints DRAFT_ID and CONTENT_HASH

# In governance-frontend: create the proposal with the frontend's gateway, signed by the CLI.
REHEARSAL_PROPOSE=1 REHEARSAL_SIGNER=gov-voter-a REHEARSAL_TITLE="Longer voting, higher quorum" \
REHEARSAL_HASH=<CONTENT_HASH> REHEARSAL_URI=http://localhost:3000/drafts/<DRAFT_ID> \
REHEARSAL_CONFIG='{"votingPeriodSecs":180,"quorumBps":3000}' REHEARSAL_OUT=<out.json> \
  pnpm exec vitest run src/lib/governance.rehearsal.test.ts
```

Then vote (after the voting delay), queue (after voting ends), execute (after the timelock), and read the new settings back:

```bash
stellar contract invoke --id gov-governance --source-account gov-voter-a --network testnet -- cast_vote --voter gov-voter-a --proposal_id <ID> --choice 1
stellar contract invoke --id gov-governance --source-account gov-deployer --network testnet -- queue --proposal_id <ID>
stellar contract invoke --id gov-governance --source-account gov-deployer --network testnet -- execute --proposal_id <ID>
stellar contract invoke --id gov-governance --source-account gov-deployer --network testnet --send=no -- get_config
# In governance-toolkit, wait for the indexed state:
node scripts/wait-indexed.mjs --proposal <ID> --state Executed
```

`--choice` is `0` Against, `1` For, `2` Abstain.

## 3. Scenario (b): treasury transfer by Invoke

```bash
GOV_FUNDER=gov-deployer GOV_AMOUNT=1000000000 scripts/fund-treasury.sh          # 100 XLM to the contract
GOV_PROPOSER=gov-voter-a GOV_RECIPIENT=gov-treasury-recipient GOV_AMOUNT=250000000 \
GOV_TITLE="Pay 25 XLM to the recipient" GOV_TEXT_FILE=<text.md> scripts/propose-invoke.sh
```

Vote For, queue, execute as above, then check the recipient's XLM balance went up by 25 XLM and the contract's went down.

## 4. Scenario (c): quorum missed

Create a proposal (for example with `propose-invoke.sh`, or a signal proposal through the CLI with `--actions '[]'`). Let only a small voter vote, so For + Abstain stays below the quorum. After the voting period the state is `2` (Defeated) and `queue` fails with `WrongState`.

## 5. Scenario (d): cancelled by its proposer

Create a proposal, then `cancel --caller <proposer> --proposal_id <ID>`. The state is `6` (Canceled); a later `execute` fails.

## 6. Scenario (e): re-deposit after the snapshot does not add power

Create a proposal. Then voter b withdraws, the tokens move to voter c, voter c deposits them, and both vote. Check with `power_at --account <b or c> --ledger <snapshot_ledger>` and the vote weights in `vote_cast` events: each counts only what they had deposited before the snapshot.

## 7. Record

Write every transaction hash, ledger and time into the repository's `docs/DEMO.md`, plus what the frontend and analytics show and whether it matches the chain.

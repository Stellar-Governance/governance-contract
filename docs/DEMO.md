# DEMO

Record of runs of the built packages against testnet. Every claim here was observed; anything not observed is listed under "Not shown".

## Phase 3 smoke run (2026-11-08)

Setup: the built toolkit (`pnpm build && pnpm start` in `governance-toolkit`, testnet values in its local `.env`) on `http://127.0.0.1:8787`; the built frontend (`pnpm build` with testnet values in `.env.local`, then `next start -p 3000`) on port 3000. Contract `CAOERDLG52UU6X5JENOXNKN4DWWJPVPNK7AD4RYXE3MA4TRBY62O5I5F`.

Observed:
- The toolkit indexed the deployment (proposal 1 and its 7 events, no gap) and reported `lastSuccessAt` in `/health`.
- Every frontend route answered HTTP 200 with the page frame (skip link, labelled main navigation, footer with the audit status) and no configuration error: `/`, `/proposals/1`, `/power`, `/drafts`, `/drafts/new`, `/drafts/<unknown id>`. The page payload carries the configured contract id and toolkit URL.
- The toolkit answers CORS for the frontend origin `http://localhost:3000` (`access-control-allow-origin` set; preflight `OPTIONS /auth/challenge` 204).
- `GET /proposals/1` from the toolkit, which the proposal page loads, returned proposal 1 as on-chain: Executed, For 5,000,000,000 and Against 3,000,000,000 (500 and 300 GOV). It has no linked text (`texts: []`), so the page would show "Text not found": the rehearsal proposal was created with the CLI from a text that was never published as a draft.
- The frontend's chain reads were checked separately against testnet (`LIVE_TESTNET=1` test in `governance-frontend/src/lib/governance.live.test.ts`): token GOV with 7 decimals, voter a's deposit, 500 GOV of power at proposal 1's snapshot, voter a's For vote, the config, and a contract error mapped to its message.

Not shown:
- Data views render in the browser (client components). No browser was run here, so the rendered lists, the proposal page with data, and the forms were not seen against the live services; they are covered by component tests with fakes.
- No wallet was connected and no transaction was sent from the frontend. A real-wallet click-through is a user action (see `docs/USER_ACTIONS.md` once written).
- Layout at 360 px in a real browser.
- The browser opens the site as `http://localhost:3000`; the toolkit's CORS list must contain that exact origin (a site opened as `http://127.0.0.1:3000` would be refused).

Both processes were stopped by PID afterwards.

## Phase 4 smoke run (2026-11-08)

Setup: the built toolkit (as above, with `http://localhost:3001` added to `TOOLKIT_CORS_ORIGINS` in its local `.env`) and the analytics static export (`NEXT_PUBLIC_TOOLKIT_URL=http://127.0.0.1:8787 pnpm build`, `out/` served by a plain static file server on port 3001).

Observed:
- `/`, `/proposals` and `/participation` of the export answered 200 with the page frame and the "Loading statistics from the toolkit…" state; the toolkit URL is built into the client chunk.
- `/stats/summary`, `/stats/proposals` and `/stats/votes` answered with CORS for `http://localhost:3001` and returned the chain's data: 1 proposal (Executed, total power 800 GOV, quorum 160 GOV, For 500, Against 300), 2 indexed votes from 2 voters, 2 accounts with a deposit, no gaps. With these numbers the site's metrics give: pass rate 100% (1 of 1 decided), turnout 100%, average participation 100%, 2 unique voters.

Not shown: the rendered pages with data in a browser (client-side loading; covered by component tests with fakes).

## Phase 5: the full scenario on testnet (2026-11-08)

Contract `CAOERDLG52UU6X5JENOXNKN4DWWJPVPNK7AD4RYXE3MA4TRBY62O5I5F`, GOV token `CCX6Y3U3DJYWK5ZM6S6VGCTUTC4SYRT73HILB2QKGHEZFAG7QJNALGO3`. Times are network close times from Horizon (UTC). Steps follow `governance-contract/scripts/e2e-testnet.md`. Settings at the start: voting delay 30 s, voting period 120 s, timelock 60 s, grace 1 day, quorum 20 %, threshold 1 GOV.

### Deposits (three voters, different amounts)

Voter a 500 GOV and voter b 300 GOV deposited on 2026-11-07 (see `docs/TESTNET.md`); voter c deposited 200 GOV: `c2dd67d54a9938deafabb680e5dc30bd3ebd104452182082ce4dad09e9f0016e` (ledger 5,624,225, 13:32:47).

### Two failed attempts, kept on the record

- **First attempt (proposals 2 to 6).** Proposals 2 (a), 3 (b), 4 (d) and 5 (e) were created, but the 120-second voting windows closed before any vote was sent, because each step was run by hand and each transaction took 25 to 50 seconds. All four ended **Defeated with no votes**; cancelling 4 was refused (`WrongState`). Proposal 6 is a duplicate of 5 made by a helper script that retried a `propose` whose first attempt had already succeeded (`d768682271bdd96d0cf9ef3aa170e302b7f14d39422688654a4b84660e2b39a0` created 5, `de5b1ca408dd71e5fc830ccbd2f12d0fb39eedef00faa81319e40b9ad3be2da0` created 6); its proposer cancelled it (`72067416532862e43307a117ef9d821bf1272d2f39ce212f83e5fdd42907a6fb`). The helper was fixed to never retry a signed transaction.
- **Second attempt (proposals 7 to 9).** Proposal 7 (a) also closed before its votes (they failed with `VotingClosed`, error 9), because the script created 8 and 9 before voting. Proposals 8 and 9 did get votes and are used below.

These six proposals stay in the data; the analytics numbers below include them.

### (a) Settings change: drafted, discussed, created from the frontend's code, executed

| Step | Transaction | Ledger | Time |
|---|---|---|---|
| Draft `rQffdR3VU6zGYE4H` written and published in the toolkit by voter a (sign-in with a SEP-53 signature from `stellar message sign`); voters b and c commented | toolkit API | | 13:49 |
| Proposal **10** created with the frontend's `createGovernanceGateway().propose`, signed through `stellar tx sign` (no browser wallet): `UpdateConfig` voting period 600 s, quorum 30 %, `description_hash` = the draft's hash `37a00b65…64be` | `d23ecddf7625851643d6cbd9e8c9436af5216ca853c7c16a140d6851e3e280ed` | 5,624,432 | 13:50:02 |
| Vote For, voter a, 500 GOV | `2d81f00862bf9b3bce4a3388890f6aa5e2a888e73ee7c4c67d8ffd41779e0205` | 5,624,447 | 13:51:17 |
| Vote For, voter b, 300 GOV | `d3a37b57df3526b586a8265b2f0b69bfd2fae5c4ac511ac88f1b9974c2b96a44` | 5,624,447 | 13:51:17 |
| Queue (by gov-deployer) | `b4eb07f76777f8064df9b8c4f79d92036b701d86cb9b124e232c644ea58d7faf` | 5,624,481 | 13:54:07 |
| Execute after the timelock (by voter c) | `83cb102c23d166f11ccbe05f8a4226edc4319a7141c013d93727fc9311ab447d` | 5,624,501 | 13:55:47 |

Settings read back with `get_config` afterwards: voting delay 30 s, **voting period 600 s**, timelock 60 s, grace 86,400 s, threshold 1 GOV, **quorum 3,000 bps**. The toolkit linked draft `rQffdR3VU6zGYE4H` to proposal 10 (`/proposals/10` lists the text with the same hash), so the frontend would show "Hash matches on-chain".

### (b) Treasury transfer by an `Invoke` action

| Step | Transaction | Ledger | Time |
|---|---|---|---|
| `fund-treasury.sh`: 100 XLM from gov-deployer to the contract | `073ae6e659b92f7351fc6ec0f89e9aed24ac36c1c71f4740791382ec18e8507b` | 5,624,250 | 13:34:52 |
| `propose-invoke.sh`: proposal **8**, native XLM `transfer(contract → gov-treasury-recipient, 25 XLM)` | `2a392802033c3de42a7a489f47e98437e97d20fb845509b1b0d5ca879251b7ee` | 5,624,345 | 13:42:47 |
| Vote For, voter a, 500 GOV | `8aca94ff8c52a9b7877e52c2e7fc98d55ba86812ae98a7b45083f8e9f96f4420` | 5,624,374 | 13:45:12 |
| Queue | `3f8af687e3f1f0952b68326b1653a635a80d5c6d1a3c17428ae163285584c6b3` | 5,624,419 | 13:48:57 |
| Execute (by voter c) | `0e253e36d4fd1166cc98c22ade47cf68c73a9bdfc451bfd45a4b808cde741bd4` | 5,624,459 | 13:52:17 |

XLM balances read before and after the execute: recipient 10,000 → **10,025 XLM**; contract 100 → **75 XLM**. The transfer needed no signature from anyone: the governance contract was the direct invoker.

### (c) A proposal that misses the quorum

Proposal **12**, created under the new settings (`5843d560ea63a510af1817e9897d2f35c4cd116e5727852ea26ced343f8ac938`, 13:59:52): total power at the snapshot 1,200 GOV, quorum 360 GOV. Only voter b voted, For 300 GOV (`fe725ee65b6730a6668a313d54fc283ebab359bdf64c16c0cc81bacdecfc7691`, 14:03:52). After the 600-second period the state was **Defeated**, and `queue` was refused in simulation (`WrongState`).

### (d) Cancelled by its proposer

Proposal **13** created by voter c (`79cbd7c8ead7acd9f3797123fbe5401f804509a8d1339a87cc00c2bb05d69152`, 14:00:42) and cancelled by voter c (`a7bd43863868ecd45d7498d25fef33699f1ab51d93b87b69fdfdb0c7f70d35aa`, 14:01:42). State: **Canceled**.

### (e) Withdrawing and re-depositing more after the snapshot adds no power

| Step | Transaction | Ledger | Time |
|---|---|---|---|
| Issuer pays voter c 100 extra GOV | `a7e2022a84cfe77926d6b7f0cf3df050fd5cadbc94ca85546fb297c83f69ca97` | 5,624,516 | 13:57:02 |
| Proposal **11** created by voter b; snapshot ledger 5,624,522. Voter c's deposit 300 GOV, `power_at(c, 5,624,522)` = 300 GOV | `2016c2ab35cf42083c9161d00fd473114c2abffe31c2309e83fa2b81389bac03` | 5,624,522 | 13:57:32 |
| Voter c withdraws 300 GOV | `59abcdc203c7fafe32931e2253ceff8c005bf0c2ffecc52f38eb229336914306` | 5,624,534 | 13:58:32 |
| Voter c re-deposits 400 GOV (deposit now 400; `power_at(c, 5,624,522)` still 300) | `8c9cd3e2770c8bcbf59881a87070ab00fe3ae9156532689ac0f0aee795c4b380` | 5,624,540 | 13:59:02 |
| Vote For, voter b, 300 GOV | `b39a84f72cebc52239aaf96b7d26d6e6a30e5af1c88bf5925d6911a593bc9a72` | 5,624,582 | 14:02:32 |
| Vote For, voter c: **weight 300 GOV, not 400** (`get_vote` → power 3,000,000,000) | `268aa234a883b68e0bdc9491d86310a336c63c6eebf7102be6b6656232d64de7` | 5,624,584 | 14:02:42 |
| Queue, then execute | `c63f25d39ee10d137e27277451b16be4a7edda30d07f86b57d71d1c7acc6fdd2`, `bf245d9b56568915127fbfcc4cb58b89206b52df5e6786f38cc954e9118a01bc` | 5,624,691 / 5,624,711 | 14:11:37 / 14:13:17 |

Proposal 9 (second attempt) shows the same on another proposal: after its snapshot (ledger 5,624,352), voter c withdrew 200 and re-deposited 300 GOV, and `power_at(c, 5,624,352)` stayed 200 GOV. Proposal 9 passed with voter b's 300 GOV (`1c5f293726d52968f1a49406d3869ee8c129fb9ed11890280c551c69a8afe1e0`), was queued (`5aaf1e542c19afaea2f1dd8eb0931e33c758b009c287a9e1575e26adb58d9202`) and executed (`d05186ed94001c13a69a122a3a34d421fa6089eea69efc8785907b53ea131f84`).

### What the toolkit, frontend data and analytics showed

The built toolkit indexed all 13 proposals with no gap. `/stats/proposals` matched the chain for every proposal (tallies, total power, quorum, state). `/accounts/<voter c>` showed deposit 400 GOV and its vote on 11 at 300 GOV. `/proposals/7` and `/proposals/10` list their drafts, and the draft for (a) has its two comments.

The analytics package's own metric functions, run on that toolkit data: **pass rate 45.5 %** (5 passed of 11 decided; the two cancelled proposals excluded), **average participation 30.2 %**, **3 unique voters**. Per proposal: #10 turnout 72.7 %, quorum exceeded by 263.6 % of the quorum; #12 turnout 25.0 %, quorum missed by 16.7 %; #8 turnout 50.0 %; #11 turnout 54.5 %. These agree with the chain numbers above, checked by hand.

Not shown: no browser rendered these pages and no browser wallet signed anything. The frontend's chain code ran in Node with the Stellar CLI as signer; the toolkit sign-ins used the CLI's SEP-53 signatures.

## Known issues (CHECKPOINT 2)

- No browser was used: no real wallet click-through, no visual check of the pages with live data, no real 360 px check.
- The rehearsal settings (120 s voting) are too short for steps run by hand; use longer periods for manual demos.
- Of the frontend write paths only `propose` ran on testnet; the others were tested with fakes.
- Testnet resets on 2026-12-16 wipe everything above; the transaction links then describe history.

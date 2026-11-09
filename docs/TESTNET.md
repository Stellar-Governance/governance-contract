# TESTNET

Network: Stellar testnet (`Test SDF Network ; September 2015`), RPC `https://soroban-testnet.stellar.org`, protocol 30 on 2026-11-07.
Testnet is scheduled to reset on **2026-12-16 17:00 UTC**; everything below is wiped then and must be recreated from the scripts.

Only public addresses and contract ids appear here. Secret keys stay in the Stellar CLI key store, referenced by name.

## Identities (created 2026-11-07, task 0.3, funded by friendbot)

| Name | Public address | Role |
|---|---|---|
| `gov-deployer` | `GC7SSCXD4FT4TGR64LKKH6O7EBR355HWKIAYCJ5LOET2TJAOWXIXFITN` | Deploys the governance contract |
| `gov-voter-a` | `GDA6HEAZTBUTUVVLTQAQURAFKXJSSXGCAURILN6QIRDFIJPLBWOLWNNT` | Voter, 1,000 GOV |
| `gov-voter-b` | `GARQQ444KKYHFWFQHRZERDNNO74IHV7JZF5HYEMQ7ZJLFCABVE4QQAEW` | Voter, 600 GOV |
| `gov-voter-c` | `GAFE6DFMIYL64SLPK6LURMWUMICBIJ7TDJIHDXMGSL7D5W3QPKLYWV4N` | Voter, 300 GOV |
| `gov-treasury-recipient` | `GB3YGACV6WQYF5WKJL5CCNQNXTJAPT24B4NCDXRJROA2ETQVV2WLWUDE` | Receives the treasury transfer in scenario (b) |
| `gov-token-issuer` | `GDTLQKDKRUJAT4OTZVU4VIPIDT3E4HJQSQ3IAC67LJ3WUBCP3IO554UI` | Issuer of the classic asset `GOV` |

## Tokens (decision D13)

| Token | Contract id | Notes |
|---|---|---|
| Governance token `GOV:GDTLQ...554UI` (SAC, CLI alias `gov-token`) | `CCX6Y3U3DJYWK5ZM6S6VGCTUTC4SYRT73HILB2QKGHEZFAG7QJNALGO3` | `decimals()` = 7, `symbol()` = `GOV`. Deployed 2026-11-07 |
| Treasury token: native XLM SAC | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` | `symbol()` returns `native`; shown as XLM |

GOV distribution transactions (2026-11-07): trustlines `d0f2fab8...ca5d` (a), `939bd3cd...74f3` (b), `68464c53...c9b9` (c); payments `4b0d206d...315c` (1,000 to a), `7baf9383...7654` (600 to b), `cfe42381...b667` (300 to c). Balances read back with `balance` on the SAC: 10000000000, 6000000000, 3000000000.

Commands used (also the redeploy recipe until task 5.1 turns it into a script):
```bash
stellar keys generate <name> --network testnet --fund
stellar contract asset deploy --asset GOV:<issuer> --alias gov-token --source-account gov-token-issuer --network testnet
stellar tx new change-trust --line GOV:<issuer> --source-account <voter> --network testnet
stellar tx new payment --destination <voter G...> --asset GOV:<issuer> --amount <stroops> --source-account gov-token-issuer --network testnet
```

## Governance contract (task 1.7, deployed 2026-11-07)

| Item | Value |
|---|---|
| Contract id (CLI alias `gov-governance`) | `CAOERDLG52UU6X5JENOXNKN4DWWJPVPNK7AD4RYXE3MA4TRBY62O5I5F` |
| Governance token | `CCX6Y3U3DJYWK5ZM6S6VGCTUTC4SYRT73HILB2QKGHEZFAG7QJNALGO3` (GOV SAC) |
| Wasm sha256 | `b08f42668fb100075b215766c5b3a278d5888fbfab57cb2c8b358fc57b1b30ef` (26,022 bytes, soroban-sdk 29.0.1, commit `f659ba0`) |
| Deployed by | `gov-deployer`, with `governance-contract/scripts/deploy-testnet.sh` |
| Network | protocol 30. SDK 29 contracts deploy and run on it (decision D12 confirmed) |

Rehearsal config, read back with `get_config`:
`{"voting_delay_secs":30,"voting_period_secs":120,"timelock_secs":60,"grace_period_secs":86400,"proposal_threshold":"10000000","quorum_bps":2000}` (threshold = 1 GOV, quorum 20 %).

### Lifecycle rehearsal (proposal 1, signal proposal, no actions)

| Step | Identity | Tx hash | Ledger | Time (UTC) | Fee charged (stroops) | of which rent |
|---|---|---|---|---|---|---|
| Upload wasm | gov-deployer | `0e21b4cd443f4e9d3211ac13a4071cffa0488424e5b703c3dc601e2280649763` | 5,610,989 | 19:09:46 | 15,961,330 | not measured |
| Deploy + constructor | gov-deployer | `8745c9761889e920c0e1ec439512d17b2789bc6f0ce38ca1329e5aab39e9ac1c` | 5,611,001 | 19:10:46 | 393,779,675 | 393,774,710 |
| `deposit` 500 GOV (first deposit) | gov-voter-a | `803e31c8f095cff6aa68ae9dd08425cbf44260a65a5518fd36745f1cdba6fb4b` | 5,611,014 | 19:11:51 | 2,017,351 | 2,000,957 |
| `deposit` 300 GOV (first deposit) | gov-voter-b | `9b6ceb46d45623e4f3946ef82aed4b488b401781a968d2845a005fec54807602` | 5,611,040 | 19:14:01 | 1,670,810 | 1,655,082 |
| `propose` "Rehearsal 1" | gov-voter-a | `69772f822668b35e1f72345d0b3d37ef6d1bacd723b017fcfc7591b037e74c28` | 5,611,046 | 19:14:31 | 1,988,965 | 1,981,271 |
| `cast_vote` For, power 500 GOV | gov-voter-a | `1e8f0ead2b269c92edbf91630462ab751fbdc2c7d1de9b5fa06bfc3357aa0af3` | 5,611,063 | 19:15:56 | 527,913 | 521,271 |
| `cast_vote` Against, power 300 GOV | gov-voter-b | `21ea6d5dca073b652df525b6bfdd2384b497785b3d3f0b207b09accde579c50c` | 5,611,070 | 19:16:31 | 527,913 | 521,271 |
| `queue` (eta 1794079116) | gov-deployer (not the proposer) | `b8f2ff96c33bd5f729690f84066b9dcd29bc1e78f60b6c09e1dbcd3841675ae0` | 5,611,083 | 19:17:36 | 3,793 | 0 |
| `execute` | gov-voter-c (did not vote) | `0b7fc8b3bb822d340bc81354ece11bd2b0be766e5ccc1e04d70e500c86f8aaaa` | 5,613,085 | 22:04:26 | 3,726 | 0 |

Read back after the run: proposal 1 had `snapshot_ledger` 5,611,046, `total_power` 8,000,000,000 (800 GOV), `quorum` 1,600,000,000 (160 GOV), For 500 GOV, Against 300 GOV; `state` went 0 Pending, 1 Active, 3 Succeeded, then 5 Executed. Each call emitted its event (shown by the CLI). `execute` ran almost three hours after `queue`, still well inside the one-day grace period.

Fee notes (1 XLM = 10,000,000 stroops; rent from `stellar tx fetch meta`, field `rent_fee_charged`):
- Rent is nearly all of the cost. Every persistent write extends the entry to the network maximum TTL (about 180 days, decision D14), so the first deposit pays about 0.2 XLM and a proposal about 0.2 XLM up front. A vote (one new entry) is about 0.05 XLM. Calls that create no entries and need no extension (`queue`, `execute` without actions) cost under 0.0004 XLM.
- The deploy paid about 39.4 XLM rent because `instance().extend_ttl` also extends the 26 KB contract code entry to the maximum TTL. When the instance next drops below the 30-day threshold (about 150 days after deploy), the caller of the next state-changing call pays a similar extension. Fine on testnet; must be revisited before any mainnet idea (Questions table, Q1).
- Inclusion fee was 100 stroops for every call.

Clock note: this machine's clock was about 73 minutes behind real time during the run, while ledger close times matched an outside clock (an HTTP `Date` header). Waits in scripts must poll ledger state or the latest ledger's close time, never the local clock.

## Redeploy drill (task 5.4, 2026-11-09)

`GOV_ALIAS=gov-governance-drill GOV_ENV_OUT=<scratch folder> governance-contract/scripts/redeploy-testnet.sh` ran in **333 s** end to end: identities checked and re-funded (171 s, friendbot), GOV SAC found (already deployed), voter balances checked (one payment to top up voter c), contract built and deployed as `CA736PCFYDMEXS5VYJQPGGBXS2HLDRW3JMJV3SXPFHUTXHI65T77AJUV` (config read back: the rehearsal defaults), env files for the toolkit, frontend and analytics written. After a real reset, run it without `GOV_ALIAS`/`GOV_ENV_OUT`, then rebuild the frontend and analytics (about a minute each) and restart the toolkit. The drill contract is not used for anything else.

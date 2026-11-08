#!/usr/bin/env bash
# Sends a treasury token (native XLM by default) to the governance contract, so a proposal can
# later move it with an Invoke action. TESTNET only.
#
# Env: GOV_FUNDER (CLI identity, required), GOV_AMOUNT (smallest units, required),
#      GOV_TOKEN (token contract id, default the native XLM SAC), GOV_CONTRACT, GOV_NETWORK.
set -euo pipefail
. "$(dirname "$0")/_common.sh"

: "${GOV_FUNDER:?set GOV_FUNDER to a Stellar CLI identity name}"
: "${GOV_AMOUNT:?set GOV_AMOUNT in smallest token units}"
TOKEN="${GOV_TOKEN:-$(native_sac)}"
GOV_ID="$(contract_id "$GOV_CONTRACT")"

stellar contract invoke --id "$TOKEN" --source-account "$GOV_FUNDER" --network "$NETWORK" -- \
  transfer --from "$GOV_FUNDER" --to "$GOV_ID" --amount "$GOV_AMOUNT"
echo "governance contract balance of $TOKEN:" >&2
stellar contract invoke --id "$TOKEN" --source-account "$GOV_FUNDER" --network "$NETWORK" --send=no -- \
  balance --id "$GOV_ID"

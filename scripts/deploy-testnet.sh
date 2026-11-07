#!/usr/bin/env bash
# Builds and deploys the governance contract to TESTNET. Refuses any other network.
#
# Required env:
#   GOV_TOKEN_ID      contract id of the governance token (a SEP-41 token, for example a SAC)
# Optional env (defaults in brackets):
#   GOV_NETWORK       [testnet]   must be "testnet"
#   GOV_DEPLOYER      [gov-deployer]  Stellar CLI identity name (never a secret key)
#   GOV_ALIAS         [gov-governance] CLI alias saved for the new contract id
#   GOV_VOTING_DELAY_SECS [30]  GOV_VOTING_PERIOD_SECS [120]  GOV_TIMELOCK_SECS [60]
#   GOV_GRACE_PERIOD_SECS [86400]  GOV_QUORUM_BPS [2000]  GOV_PROPOSAL_THRESHOLD [10000000]
#
# Prints the new contract id on the last line of stdout.
set -euo pipefail

NETWORK="${GOV_NETWORK:-testnet}"
if [ "$NETWORK" != "testnet" ]; then
  echo "refusing to deploy: GOV_NETWORK is '$NETWORK', only 'testnet' is allowed" >&2
  exit 2
fi
: "${GOV_TOKEN_ID:?set GOV_TOKEN_ID to the governance token contract id}"
DEPLOYER="${GOV_DEPLOYER:-gov-deployer}"
ALIAS="${GOV_ALIAS:-gov-governance}"

CONFIG=$(printf '{"voting_delay_secs":%s,"voting_period_secs":%s,"timelock_secs":%s,"grace_period_secs":%s,"proposal_threshold":"%s","quorum_bps":%s}' \
  "${GOV_VOTING_DELAY_SECS:-30}" "${GOV_VOTING_PERIOD_SECS:-120}" "${GOV_TIMELOCK_SECS:-60}" \
  "${GOV_GRACE_PERIOD_SECS:-86400}" "${GOV_PROPOSAL_THRESHOLD:-10000000}" "${GOV_QUORUM_BPS:-2000}")

cd "$(dirname "$0")/.."
stellar contract build >&2
WASM=target/wasm32v1-none/release/governance_contract.wasm
echo "wasm sha256: $(sha256sum "$WASM" | cut -d' ' -f1)" >&2
echo "config: $CONFIG" >&2

stellar contract deploy \
  --wasm "$WASM" \
  --source-account "$DEPLOYER" \
  --network "$NETWORK" \
  --alias "$ALIAS" \
  -- \
  --token "$GOV_TOKEN_ID" \
  --config "$CONFIG"

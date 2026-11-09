#!/usr/bin/env bash
# Rebuilds the whole testnet setup after a testnet reset (or for a drill): identities, the GOV
# asset and its SAC, voter balances, the governance contract, and the env files of the other
# packages. TESTNET only. Identities are created in the Stellar CLI key store if missing; no
# secret is ever printed.
#
# Env: GOV_ALIAS (CLI alias for the new contract, default gov-governance),
#      GOV_ENV_OUT (folder that holds the governance-toolkit, governance-frontend and
#      voting-analytics checkouts whose env files are written, default
#      the parent folder of this checkout; set it elsewhere for a drill),
#      plus the config variables of deploy-testnet.sh.
set -euo pipefail
. "$(dirname "$0")/_common.sh"
ALIAS="${GOV_ALIAS:-gov-governance}"
HERE="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${GOV_ENV_OUT:-$(dirname "$HERE")}"
START=$(date +%s)

step() { echo "[$(( $(date +%s) - START ))s] $*" >&2; }

for id in gov-deployer gov-voter-a gov-voter-b gov-voter-c gov-treasury-recipient gov-token-issuer; do
  if stellar keys address "$id" >/dev/null 2>&1; then
    # After a reset the key exists but the account does not: fund it again (friendbot ignores
    # accounts that already exist).
    stellar keys fund "$id" --network "$NETWORK" >/dev/null 2>&1 || true
  else
    stellar keys generate "$id" --network "$NETWORK" --fund >/dev/null
  fi
done
step "identities ready"

ISSUER=$(stellar keys address gov-token-issuer)
ASSET="GOV:$ISSUER"
TOKEN=$(stellar contract id asset --asset "$ASSET" --network "$NETWORK")
if ! stellar contract invoke --id "$TOKEN" --source-account gov-deployer --network "$NETWORK" --send=no -- decimals >/dev/null 2>&1; then
  stellar contract asset deploy --asset "$ASSET" --alias gov-token --source-account gov-token-issuer --network "$NETWORK" >/dev/null
fi
step "GOV SAC $TOKEN"

for pair in "gov-voter-a 10000000000" "gov-voter-b 6000000000" "gov-voter-c 3000000000"; do
  set -- $pair
  stellar tx new change-trust --line "$ASSET" --source-account "$1" --network "$NETWORK" >/dev/null 2>&1 || true
  bal=$(stellar contract invoke --id "$TOKEN" --source-account "$1" --network "$NETWORK" --send=no -- balance --id "$1" 2>/dev/null | tr -d '"')
  if [ "${bal:-0}" = "0" ]; then
    stellar tx new payment --destination "$(stellar keys address "$1")" --asset "$ASSET" --amount "$2" \
      --source-account gov-token-issuer --network "$NETWORK" >/dev/null
  fi
done
step "voter balances ready"

CONTRACT=$(GOV_TOKEN_ID="$TOKEN" GOV_ALIAS="$ALIAS" "$HERE/scripts/deploy-testnet.sh" 2>/dev/null | tail -1)
LEDGER=$(curl -s -m 30 -X POST "https://soroban-testnet.stellar.org" -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' | sed -n 's/.*"sequence":\([0-9]*\).*/\1/p')
step "governance contract $CONTRACT (start ledger about $LEDGER)"

PASS="Test SDF Network ; September 2015"
RPC="https://soroban-testnet.stellar.org"
mkdir -p "$OUT/governance-toolkit" "$OUT/governance-frontend" "$OUT/voting-analytics"
cat > "$OUT/governance-toolkit/.env" <<EOF
STELLAR_NETWORK=testnet
STELLAR_NETWORK_PASSPHRASE=$PASS
STELLAR_RPC_URL=$RPC
GOVERNANCE_CONTRACT_ID=$CONTRACT
TOOLKIT_START_LEDGER=$LEDGER
TOOLKIT_AUTH_DOMAIN=localhost
TOOLKIT_CORS_ORIGINS=http://localhost:3000,http://localhost:3001
EOF
cat > "$OUT/governance-frontend/.env.local" <<EOF
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE=$PASS
NEXT_PUBLIC_STELLAR_RPC_URL=$RPC
NEXT_PUBLIC_GOVERNANCE_CONTRACT_ID=$CONTRACT
NEXT_PUBLIC_TOOLKIT_URL=http://127.0.0.1:8787
EOF
printf 'NEXT_PUBLIC_TOOLKIT_URL=http://127.0.0.1:8787\n' > "$OUT/voting-analytics/.env.local"
step "env files written under $OUT (toolkit .env, frontend and analytics .env.local)"
echo "CONTRACT=$CONTRACT"
echo "TOKEN=$TOKEN"

#!/usr/bin/env bash
# Creates a treasury proposal: one Invoke action that makes the governance contract call
# `transfer(governance, recipient, amount)` on a token it holds. TESTNET only.
#
# Env: GOV_PROPOSER (CLI identity, required), GOV_RECIPIENT (identity or address, required),
#      GOV_AMOUNT (smallest units, required), GOV_TEXT_FILE (the proposal text, required; its
#      sha256 becomes description_hash), GOV_TITLE (default "Treasury transfer"),
#      GOV_URI (where the text can be read; default "file:<name>"), GOV_TOKEN (default native XLM),
#      GOV_CONTRACT, GOV_NETWORK.
# Prints the new proposal id on the last line.
set -euo pipefail
. "$(dirname "$0")/_common.sh"

: "${GOV_PROPOSER:?set GOV_PROPOSER to a Stellar CLI identity name}"
: "${GOV_RECIPIENT:?set GOV_RECIPIENT to an identity name or address}"
: "${GOV_AMOUNT:?set GOV_AMOUNT in smallest token units}"
: "${GOV_TEXT_FILE:?set GOV_TEXT_FILE to the proposal text}"
if ! [[ "$GOV_AMOUNT" =~ ^[0-9]+$ ]]; then
  echo "GOV_AMOUNT must be a whole number of smallest units" >&2
  exit 2
fi
TITLE="${GOV_TITLE:-Treasury transfer}"
URI="${GOV_URI:-file:$(basename "$GOV_TEXT_FILE")}"
TOKEN="${GOV_TOKEN:-$(native_sac)}"
GOV_ID="$(contract_id "$GOV_CONTRACT")"
RECIPIENT="$(address_of "$GOV_RECIPIENT")"
# sha256 of the file's exact bytes: the same value the toolkit computes for a draft body.
HASH="$(sha256sum "$GOV_TEXT_FILE" | cut -d' ' -f1)"

# Vec<Val> arguments are given as ScVal JSON.
ACTIONS=$(printf '[{"Invoke":{"contract":"%s","function":"transfer","args":[{"address":"%s"},{"address":"%s"},{"i128":"%s"}]}}]' \
  "$TOKEN" "$GOV_ID" "$RECIPIENT" "$GOV_AMOUNT")
TITLE_JSON=$(printf '%s' "$TITLE" | sed 's/\\/\\\\/g; s/"/\\"/g')
URI_JSON=$(printf '%s' "$URI" | sed 's/\\/\\\\/g; s/"/\\"/g')

echo "description_hash: $HASH" >&2
echo "action: $TOKEN.transfer($GOV_ID -> $RECIPIENT, $GOV_AMOUNT)" >&2
stellar contract invoke --id "$GOV_ID" --source-account "$GOV_PROPOSER" --network "$NETWORK" -- \
  propose --proposer "$GOV_PROPOSER" --title "\"$TITLE_JSON\"" --description_hash "$HASH" \
  --uri "\"$URI_JSON\"" --actions "$ACTIONS"

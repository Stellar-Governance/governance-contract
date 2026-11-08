#!/usr/bin/env bash
# Shared settings for the rehearsal scripts. Source it; do not run it.
# Identities are Stellar CLI key names; no secret is ever passed on the command line.
set -euo pipefail

NETWORK="${GOV_NETWORK:-testnet}"
if [ "$NETWORK" != "testnet" ]; then
  echo "refusing: GOV_NETWORK is '$NETWORK', only 'testnet' is allowed" >&2
  exit 2
fi
# Governance contract id or CLI alias (deploy-testnet.sh saves the alias gov-governance).
GOV_CONTRACT="${GOV_CONTRACT:-gov-governance}"

# Contract id of a CLI alias or id.
contract_id() {
  case "$1" in
    C*) if [ "${#1}" -eq 56 ]; then echo "$1"; return; fi ;;
  esac
  stellar contract alias show "$1" --network "$NETWORK"
}

# Public address of a CLI identity name, or the value itself if it is already an address.
address_of() {
  case "$1" in
    G* | C*) if [ "${#1}" -eq 56 ]; then echo "$1"; return; fi ;;
  esac
  stellar keys address "$1"
}

# The native XLM Stellar Asset Contract on this network.
native_sac() {
  stellar contract id asset --asset native --network "$NETWORK"
}

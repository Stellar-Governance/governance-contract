//! Storage keys, limits and TTL helpers (decision D14).

use soroban_sdk::{contracttype, Address, Env, IntoVal, TryFromVal, Val};

/// Ledgers in one day at 5 s per ledger.
pub const DAY_LEDGERS: u32 = 17_280;
/// Extend an entry's TTL when it has fewer than this many ledgers left (30 days).
pub const TTL_THRESHOLD: u32 = 30 * DAY_LEDGERS;

/// Every storage key the contract uses.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Instance: current [`crate::Config`].
    Config,
    /// Instance: governance token address.
    Token,
    /// Instance: number of proposals created (ids are 1..=count).
    ProposalCount,
    /// Persistent: proposal by id.
    Proposal(u64),
    /// Persistent: vote record by proposal id and voter.
    Vote(u64, Address),
    /// Persistent: current deposit of an account.
    Deposit(Address),
    /// Persistent: number of checkpoints of an account.
    CkptCount(Address),
    /// Persistent: checkpoint `i` (0-based) of an account.
    Ckpt(Address, u32),
    /// Persistent: number of total-power checkpoints.
    TotalCkptCount,
    /// Persistent: total-power checkpoint `i` (0-based).
    TotalCkpt(u32),
}

/// Extends the instance entry (config, token, counters).
pub fn bump_instance(env: &Env) {
    let max = env.storage().max_ttl();
    env.storage().instance().extend_ttl(TTL_THRESHOLD, max);
}

/// Reads a persistent entry and extends its TTL if it exists.
pub fn get_persistent<V: TryFromVal<Env, Val>>(env: &Env, key: &DataKey) -> Option<V> {
    let storage = env.storage().persistent();
    let value = storage.get::<DataKey, V>(key);
    if value.is_some() {
        storage.extend_ttl(key, TTL_THRESHOLD, env.storage().max_ttl());
    }
    value
}

/// Writes a persistent entry and extends its TTL to the maximum.
pub fn set_persistent<V: IntoVal<Env, Val>>(env: &Env, key: &DataKey, value: &V) {
    let storage = env.storage().persistent();
    storage.set(key, value);
    storage.extend_ttl(key, TTL_THRESHOLD, env.storage().max_ttl());
}

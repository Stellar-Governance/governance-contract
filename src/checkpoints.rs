//! Per-account and total voting power checkpoints, one storage entry each (no growing lists).
//!
//! A checkpoint `{ledger, power}` means "from `ledger` on, power is `power`". Lookups return the
//! power from changes in ledgers strictly before the requested ledger, so a proposal created in
//! ledger L sees nothing that happened in L or later.

use soroban_sdk::{Address, Env};

use crate::storage::{self, DataKey};
use crate::Checkpoint;

/// Which series of checkpoints to use.
#[derive(Clone)]
pub enum Series {
    Account(Address),
    Total,
}

impl Series {
    fn count_key(&self) -> DataKey {
        match self {
            Series::Account(a) => DataKey::CkptCount(a.clone()),
            Series::Total => DataKey::TotalCkptCount,
        }
    }

    fn item_key(&self, index: u32) -> DataKey {
        match self {
            Series::Account(a) => DataKey::Ckpt(a.clone(), index),
            Series::Total => DataKey::TotalCkpt(index),
        }
    }
}

fn count(env: &Env, series: &Series, bump: bool) -> u32 {
    let key = series.count_key();
    if bump {
        storage::get_persistent(env, &key).unwrap_or(0)
    } else {
        env.storage().persistent().get(&key).unwrap_or(0)
    }
}

fn item(env: &Env, series: &Series, index: u32, bump: bool) -> Option<Checkpoint> {
    let key = series.item_key(index);
    if bump {
        storage::get_persistent(env, &key)
    } else {
        env.storage().persistent().get(&key)
    }
}

/// Latest power of a series (0 when it has no checkpoints).
pub fn latest(env: &Env, series: &Series) -> i128 {
    let n = count(env, series, false);
    if n == 0 {
        return 0;
    }
    item(env, series, n - 1, false).map_or(0, |c| c.power)
}

/// Records `power` as the series' value from the current ledger on.
/// A checkpoint already written in this ledger is overwritten instead of adding one.
pub fn write(env: &Env, series: &Series, power: i128) {
    let ledger = env.ledger().sequence();
    let n = count(env, series, true);
    let checkpoint = Checkpoint { ledger, power };
    if n > 0 {
        if let Some(last) = item(env, series, n - 1, true) {
            if last.ledger == ledger {
                storage::set_persistent(env, &series.item_key(n - 1), &checkpoint);
                return;
            }
        }
    }
    storage::set_persistent(env, &series.item_key(n), &checkpoint);
    storage::set_persistent(env, &series.count_key(), &(n + 1));
}

/// Power from changes in ledgers strictly before `ledger`. Binary search over the checkpoints.
/// `bump` extends the TTL of every entry read (used by state-changing calls, decision D14).
pub fn power_before(env: &Env, series: &Series, ledger: u32, bump: bool) -> i128 {
    // Find the number of checkpoints with `checkpoint.ledger < ledger`; they are sorted by ledger.
    let mut low: u32 = 0;
    let mut high: u32 = count(env, series, bump);
    while low < high {
        let mid = low + (high - low) / 2;
        match item(env, series, mid, bump) {
            Some(c) if c.ledger < ledger => low = mid + 1,
            _ => high = mid,
        }
    }
    if low == 0 {
        return 0;
    }
    item(env, series, low - 1, bump).map_or(0, |c| c.power)
}

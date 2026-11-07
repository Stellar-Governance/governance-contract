//! Stellar Governance contract.
//!
//! Holders deposit one SEP-41 token to get voting power, which is checkpointed per ledger.
//! Proposals read power at their creation ledger, pass by quorum and majority, wait out a
//! timelock and are executed by anyone. There is no admin key. Not audited.
#![no_std]

mod config;
mod error;
mod storage;
mod types;

pub use config::{
    DAY_SECS, MAX_BPS, MAX_DURATION_SECS, MIN_GRACE_PERIOD_SECS, MIN_VOTING_PERIOD_SECS,
};
pub use error::Error;
pub use storage::DataKey;
pub use types::{
    Action, Call, Checkpoint, Config, Proposal, ProposalState, VoteChoice, VoteRecord,
};

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Vec};

/// Most actions one proposal may carry.
pub const MAX_ACTIONS: u32 = 5;
/// Most arguments one `Invoke` action may carry.
pub const MAX_ARGS: u32 = 8;
/// Longest title, in bytes.
pub const MAX_TITLE_BYTES: u32 = 120;
/// Longest URI, in bytes.
pub const MAX_URI_BYTES: u32 = 200;

#[contract]
pub struct Governance;

#[contractimpl]
impl Governance {
    /// Binds the contract to `token` and stores `config`. Runs once, at deploy. No admin is stored.
    pub fn __constructor(env: Env, token: Address, config: Config) -> Result<(), Error> {
        config::validate(&config)?;
        let instance = env.storage().instance();
        instance.set(&DataKey::Token, &token);
        instance.set(&DataKey::Config, &config);
        instance.set(&DataKey::ProposalCount, &0u64);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Moves `amount` of the token from `from` into the contract and raises its voting power.
    /// Returns the new deposit.
    pub fn deposit(_env: Env, _from: Address, _amount: i128) -> Result<i128, Error> {
        Err(Error::WrongState) // implemented in task 1.2
    }

    /// Returns `amount` of the deposit to `from`. Lowers power only for later snapshots.
    /// Returns the remaining deposit.
    pub fn withdraw(_env: Env, _from: Address, _amount: i128) -> Result<i128, Error> {
        Err(Error::WrongState) // implemented in task 1.2
    }

    /// Current deposit of `account` (0 if none).
    pub fn deposit_of(_env: Env, _account: Address) -> i128 {
        0 // implemented in task 1.2
    }

    /// Power of `account` from all changes in ledgers strictly before `ledger`.
    pub fn power_at(_env: Env, _account: Address, _ledger: u32) -> Result<i128, Error> {
        Err(Error::WrongState) // implemented in task 1.2
    }

    /// Total deposited power from all changes in ledgers strictly before `ledger`.
    pub fn total_power_at(_env: Env, _ledger: u32) -> Result<i128, Error> {
        Err(Error::WrongState) // implemented in task 1.2
    }

    /// Creates a proposal and returns its id (ids start at 1).
    pub fn propose(
        _env: Env,
        _proposer: Address,
        _title: String,
        _description_hash: BytesN<32>,
        _uri: String,
        _actions: Vec<Action>,
    ) -> Result<u64, Error> {
        Err(Error::WrongState) // implemented in task 1.3
    }

    /// Casts `voter`'s vote and returns the power counted.
    pub fn cast_vote(
        _env: Env,
        _voter: Address,
        _proposal_id: u64,
        _choice: VoteChoice,
    ) -> Result<i128, Error> {
        Err(Error::WrongState) // implemented in task 1.4
    }

    /// Cancels a proposal. Only its proposer, and only before execution.
    pub fn cancel(_env: Env, _caller: Address, _proposal_id: u64) -> Result<(), Error> {
        Err(Error::WrongState) // implemented in task 1.5
    }

    /// Queues a Succeeded proposal. Anyone may call. Returns the eta.
    pub fn queue(_env: Env, _proposal_id: u64) -> Result<u64, Error> {
        Err(Error::WrongState) // implemented in task 1.5
    }

    /// Executes a Queued proposal inside its window. Anyone may call.
    pub fn execute(_env: Env, _proposal_id: u64) -> Result<(), Error> {
        Err(Error::WrongState) // implemented in task 1.5
    }

    /// Current state of a proposal.
    pub fn state(_env: Env, _proposal_id: u64) -> Result<ProposalState, Error> {
        Err(Error::ProposalNotFound) // implemented in task 1.3
    }

    /// A stored proposal.
    pub fn get_proposal(_env: Env, _proposal_id: u64) -> Result<Proposal, Error> {
        Err(Error::ProposalNotFound) // implemented in task 1.3
    }

    /// How `voter` voted on a proposal, if at all.
    pub fn get_vote(_env: Env, _proposal_id: u64, _voter: Address) -> Option<VoteRecord> {
        None // implemented in task 1.4
    }

    /// Current settings.
    pub fn get_config(env: Env) -> Config {
        // Set by the constructor, so always present.
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap_or(Config {
                voting_delay_secs: 0,
                voting_period_secs: 0,
                timelock_secs: 0,
                grace_period_secs: 0,
                proposal_threshold: 0,
                quorum_bps: 0,
            })
    }

    /// The governance token.
    pub fn token(env: Env) -> Address {
        // Set by the constructor, so always present; the fallback is never reached.
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .unwrap_or(env.current_contract_address())
    }

    /// Number of proposals created so far.
    pub fn proposal_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test;

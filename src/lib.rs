//! Stellar Governance contract.
//!
//! Holders deposit one SEP-41 token to get voting power, which is checkpointed per ledger.
//! Proposals read power at their creation ledger, pass by quorum and majority, wait out a
//! timelock and are executed by anyone. There is no admin key. Not audited.
#![no_std]

mod checkpoints;
mod config;
mod error;
mod events;
mod proposals;
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

use checkpoints::Series;
use soroban_sdk::{
    contract, contractimpl, token, Address, BytesN, Env, MuxedAddress, String, Val, Vec,
};

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
    /// Returns the new deposit. Needs `from`'s authorization.
    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<i128, Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        from.require_auth();
        let old = read_deposit(&env, &from);
        let new = old.checked_add(amount).ok_or(Error::Overflow)?;
        let total = checkpoints::latest(&env, &Series::Total)
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        let contract = env.current_contract_address();
        transfer_token(&env, &from, &contract, amount)?;
        set_power(&env, &from, new, total);
        events::Deposited {
            account: from,
            amount,
            deposit: new,
            total,
        }
        .publish(&env);
        Ok(new)
    }

    /// Returns `amount` of the deposit to `from`. Allowed at any time: it lowers power only for
    /// snapshots taken after this ledger, and votes already cast stay counted.
    /// Returns the remaining deposit. Needs `from`'s authorization.
    pub fn withdraw(env: Env, from: Address, amount: i128) -> Result<i128, Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        from.require_auth();
        let old = read_deposit(&env, &from);
        if amount > old {
            return Err(Error::InsufficientDeposit);
        }
        let new = old - amount;
        let total = checkpoints::latest(&env, &Series::Total)
            .checked_sub(amount)
            .ok_or(Error::Overflow)?;
        let contract = env.current_contract_address();
        transfer_token(&env, &contract, &from, amount)?;
        set_power(&env, &from, new, total);
        events::Withdrawn {
            account: from,
            amount,
            deposit: new,
            total,
        }
        .publish(&env);
        Ok(new)
    }

    /// Current deposit of `account` (0 if none).
    pub fn deposit_of(env: Env, account: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Deposit(account))
            .unwrap_or(0)
    }

    /// Power of `account` from all changes in ledgers strictly before `ledger`.
    /// `ledger` must not be after the current ledger (`SnapshotNotPast`).
    pub fn power_at(env: Env, account: Address, ledger: u32) -> Result<i128, Error> {
        check_past(&env, ledger)?;
        Ok(checkpoints::power_before(
            &env,
            &Series::Account(account),
            ledger,
            false,
        ))
    }

    /// Total deposited power from all changes in ledgers strictly before `ledger`.
    /// `ledger` must not be after the current ledger (`SnapshotNotPast`).
    pub fn total_power_at(env: Env, ledger: u32) -> Result<i128, Error> {
        check_past(&env, ledger)?;
        Ok(checkpoints::power_before(
            &env,
            &Series::Total,
            ledger,
            false,
        ))
    }

    /// Creates a proposal and returns its id (ids start at 1). Needs `proposer`'s authorization.
    ///
    /// The snapshot is the current ledger: power and total power come from earlier ledgers.
    /// The proposer's power there must reach the threshold. Title <= 120 bytes, uri <= 200 bytes,
    /// at most 5 actions and 8 arguments per `Invoke`; `UpdateConfig` is validated now and again
    /// at execution; an `Invoke` on the governance token or this contract is `ForbiddenTarget`.
    pub fn propose(
        env: Env,
        proposer: Address,
        title: String,
        description_hash: BytesN<32>,
        uri: String,
        actions: Vec<Action>,
    ) -> Result<u64, Error> {
        proposer.require_auth();
        let token = Self::token(env.clone());
        proposals::validate(
            &title,
            &uri,
            &actions,
            &token,
            &env.current_contract_address(),
        )?;
        let config = Self::get_config(env.clone());
        let snapshot_ledger = env.ledger().sequence();
        let power = checkpoints::power_before(
            &env,
            &Series::Account(proposer.clone()),
            snapshot_ledger,
            true,
        );
        if power < config.proposal_threshold {
            return Err(Error::BelowThreshold);
        }
        let total_power = checkpoints::power_before(&env, &Series::Total, snapshot_ledger, true);
        let quorum = proposals::quorum(total_power, config.quorum_bps)?;
        let now = env.ledger().timestamp();
        let vote_start = now
            .checked_add(config.voting_delay_secs)
            .ok_or(Error::Overflow)?;
        let vote_end = vote_start
            .checked_add(config.voting_period_secs)
            .ok_or(Error::Overflow)?;
        let id = Self::proposal_count(env.clone())
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        let proposal = Proposal {
            proposer: proposer.clone(),
            title,
            description_hash: description_hash.clone(),
            uri,
            actions,
            snapshot_ledger,
            vote_start,
            vote_end,
            total_power,
            quorum,
            timelock_secs: config.timelock_secs,
            grace_period_secs: config.grace_period_secs,
            for_votes: 0,
            against_votes: 0,
            abstain_votes: 0,
            eta: 0,
            executed: false,
            canceled: false,
        };
        proposals::save(&env, id, &proposal);
        env.storage().instance().set(&DataKey::ProposalCount, &id);
        storage::bump_instance(&env);
        events::ProposalCreated {
            proposal_id: id,
            proposer,
            snapshot_ledger,
            vote_start,
            vote_end,
            description_hash,
            total_power,
            quorum,
        }
        .publish(&env);
        Ok(id)
    }

    /// Casts `voter`'s vote and returns the power counted. Needs `voter`'s authorization.
    /// Only while Active (`vote_start <= now < vote_end`). The weight is the voter's power at the
    /// proposal's snapshot and must be > 0. One vote per voter; it cannot be changed.
    pub fn cast_vote(
        env: Env,
        voter: Address,
        proposal_id: u64,
        choice: VoteChoice,
    ) -> Result<i128, Error> {
        voter.require_auth();
        let mut p = proposals::load(&env, proposal_id)?;
        if proposals::state(&p, env.ledger().timestamp()) != ProposalState::Active {
            return Err(Error::VotingClosed);
        }
        let vote_key = DataKey::Vote(proposal_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(Error::AlreadyVoted);
        }
        let power = checkpoints::power_before(
            &env,
            &Series::Account(voter.clone()),
            p.snapshot_ledger,
            true,
        );
        // WAVE-ISSUE(contract-2): a quadratic mode would derive the weight from `power` here.
        let weight = power;
        if weight <= 0 {
            return Err(Error::NoVotingPower);
        }
        let tally = match choice {
            VoteChoice::For => &mut p.for_votes,
            VoteChoice::Against => &mut p.against_votes,
            VoteChoice::Abstain => &mut p.abstain_votes,
        };
        *tally = tally.checked_add(weight).ok_or(Error::Overflow)?;
        storage::set_persistent(
            &env,
            &vote_key,
            &VoteRecord {
                choice,
                power: weight,
            },
        );
        proposals::save(&env, proposal_id, &p);
        storage::bump_instance(&env);
        events::VoteCast {
            proposal_id,
            voter,
            choice,
            power: weight,
        }
        .publish(&env);
        Ok(weight)
    }

    /// Cancels a proposal. Needs `caller`'s authorization, and `caller` must be the proposer.
    /// Allowed while Pending, Active, Succeeded or Queued.
    pub fn cancel(env: Env, caller: Address, proposal_id: u64) -> Result<(), Error> {
        caller.require_auth();
        let mut p = proposals::load(&env, proposal_id)?;
        // WAVE-ISSUE(contract-3): a guardian address from the config would also be accepted here.
        if caller != p.proposer {
            return Err(Error::NotAuthorized);
        }
        match proposals::state(&p, env.ledger().timestamp()) {
            ProposalState::Pending
            | ProposalState::Active
            | ProposalState::Succeeded
            | ProposalState::Queued => {}
            _ => return Err(Error::WrongState),
        }
        p.canceled = true;
        proposals::save(&env, proposal_id, &p);
        storage::bump_instance(&env);
        events::ProposalCanceled { proposal_id }.publish(&env);
        Ok(())
    }

    /// Queues a Succeeded proposal: `eta = now + timelock`. Anyone may call. Returns the eta.
    pub fn queue(env: Env, proposal_id: u64) -> Result<u64, Error> {
        let mut p = proposals::load(&env, proposal_id)?;
        let now = env.ledger().timestamp();
        if proposals::state(&p, now) != ProposalState::Succeeded {
            return Err(Error::WrongState);
        }
        let eta = now.checked_add(p.timelock_secs).ok_or(Error::Overflow)?;
        p.eta = eta;
        proposals::save(&env, proposal_id, &p);
        storage::bump_instance(&env);
        events::ProposalQueued { proposal_id, eta }.publish(&env);
        Ok(eta)
    }

    /// Executes a Queued proposal when `eta <= now < eta + grace`. Anyone may call.
    /// Marks it executed, then runs its actions in order. If any action fails the call returns
    /// `ActionFailed` and every change, including the executed flag, is rolled back.
    pub fn execute(env: Env, proposal_id: u64) -> Result<(), Error> {
        let mut p = proposals::load(&env, proposal_id)?;
        let now = env.ledger().timestamp();
        if proposals::state(&p, now) != ProposalState::Queued {
            return Err(Error::WrongState);
        }
        if now < p.eta {
            return Err(Error::TimelockActive);
        }
        p.executed = true;
        proposals::save(&env, proposal_id, &p);
        for action in p.actions.iter() {
            match action {
                Action::UpdateConfig(config) => {
                    config::validate(&config).map_err(|_| Error::ActionFailed)?;
                    env.storage().instance().set(&DataKey::Config, &config);
                    events::ConfigUpdated {
                        proposal_id,
                        config,
                    }
                    .publish(&env);
                }
                Action::Invoke(call) => {
                    let result = env.try_invoke_contract::<Val, soroban_sdk::Error>(
                        &call.contract,
                        &call.function,
                        call.args,
                    );
                    if !matches!(result, Ok(Ok(_))) {
                        return Err(Error::ActionFailed);
                    }
                }
            }
        }
        storage::bump_instance(&env);
        events::ProposalExecuted { proposal_id }.publish(&env);
        Ok(())
    }

    /// Current state of a proposal, derived from the ledger time and its flags.
    pub fn state(env: Env, proposal_id: u64) -> Result<ProposalState, Error> {
        let p = proposals::peek(&env, proposal_id)?;
        Ok(proposals::state(&p, env.ledger().timestamp()))
    }

    /// A stored proposal.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, Error> {
        proposals::peek(&env, proposal_id)
    }

    /// How `voter` voted on a proposal, if at all.
    pub fn get_vote(env: Env, proposal_id: u64, voter: Address) -> Option<VoteRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Vote(proposal_id, voter))
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

fn read_deposit(env: &Env, account: &Address) -> i128 {
    storage::get_persistent(env, &DataKey::Deposit(account.clone())).unwrap_or(0)
}

/// Stores the new deposit and writes the account and total checkpoints.
// WAVE-ISSUE(contract-1): with delegation, the account checkpoint would move to the delegatee here.
fn set_power(env: &Env, account: &Address, deposit: i128, total: i128) {
    storage::set_persistent(env, &DataKey::Deposit(account.clone()), &deposit);
    checkpoints::write(env, &Series::Account(account.clone()), deposit);
    checkpoints::write(env, &Series::Total, total);
    storage::bump_instance(env);
}

/// Transfers the governance token. Any failure (including the token's own error codes, which
/// could collide with ours) becomes `TokenFailed`.
fn transfer_token(env: &Env, from: &Address, to: &Address, amount: i128) -> Result<(), Error> {
    let token = Governance::token(env.clone());
    let client = token::TokenClient::new(env, &token);
    match client.try_transfer(from, MuxedAddress::from(to.clone()), &amount) {
        Ok(Ok(())) => Ok(()),
        _ => Err(Error::TokenFailed),
    }
}

fn check_past(env: &Env, ledger: u32) -> Result<(), Error> {
    if ledger > env.ledger().sequence() {
        Err(Error::SnapshotNotPast)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test;

//! Proposal validation, quorum and the derived state machine.

use soroban_sdk::{Address, Env, String, Vec};

use crate::{
    config, Action, Error, Proposal, ProposalState, MAX_ACTIONS, MAX_ARGS, MAX_BPS,
    MAX_TITLE_BYTES, MAX_URI_BYTES,
};

/// Checks text lengths and every action. `token` and `this` are forbidden `Invoke` targets.
pub fn validate(
    title: &String,
    uri: &String,
    actions: &Vec<Action>,
    token: &Address,
    this: &Address,
) -> Result<(), Error> {
    if title.len() > MAX_TITLE_BYTES || uri.len() > MAX_URI_BYTES {
        return Err(Error::TextTooLong);
    }
    if actions.len() > MAX_ACTIONS {
        return Err(Error::TooManyActions);
    }
    for action in actions.iter() {
        match action {
            Action::UpdateConfig(c) => config::validate(&c)?,
            Action::Invoke(call) => {
                if call.args.len() > MAX_ARGS {
                    return Err(Error::TooManyActions);
                }
                // Deposits must never be movable by a proposal (decision D4).
                if call.contract == *token || call.contract == *this {
                    return Err(Error::ForbiddenTarget);
                }
            }
        }
    }
    Ok(())
}

/// `ceil(total * bps / 10_000)`, at least 1 when `total > 0`, 0 when `total == 0`.
pub fn quorum(total: i128, bps: u32) -> Result<i128, Error> {
    if total <= 0 {
        return Ok(0);
    }
    let scaled = total.checked_mul(i128::from(bps)).ok_or(Error::Overflow)?;
    let denom = i128::from(MAX_BPS);
    // Ceiling division written as quotient + (remainder != 0), so it cannot overflow.
    let q = scaled / denom + i128::from(scaled % denom != 0);
    Ok(q.max(1))
}

/// Whether voting has ended with a pass: For beats Against and For + Abstain reach quorum.
/// A proposal with no power at its snapshot can never pass.
pub fn passed(p: &Proposal) -> bool {
    p.total_power > 0
        && p.for_votes > p.against_votes
        && p.for_votes.saturating_add(p.abstain_votes) >= p.quorum
}

/// The state of `p` at time `now`.
pub fn state(p: &Proposal, now: u64) -> ProposalState {
    if p.canceled {
        ProposalState::Canceled
    } else if p.executed {
        ProposalState::Executed
    } else if p.eta != 0 {
        if now >= p.eta.saturating_add(p.grace_period_secs) {
            ProposalState::Expired
        } else {
            ProposalState::Queued
        }
    } else if now < p.vote_start {
        ProposalState::Pending
    } else if now < p.vote_end {
        ProposalState::Active
    } else if passed(p) {
        ProposalState::Succeeded
    } else {
        ProposalState::Defeated
    }
}

/// Reads a proposal for a state-changing call (extends its TTL, decision D14).
pub fn load(env: &Env, id: u64) -> Result<Proposal, Error> {
    crate::storage::get_persistent(env, &crate::DataKey::Proposal(id))
        .ok_or(Error::ProposalNotFound)
}

/// Reads a proposal for a view call (no TTL change).
pub fn peek(env: &Env, id: u64) -> Result<Proposal, Error> {
    env.storage()
        .persistent()
        .get(&crate::DataKey::Proposal(id))
        .ok_or(Error::ProposalNotFound)
}

/// Stores a proposal (extends its TTL).
pub fn save(env: &Env, id: u64, p: &Proposal) {
    crate::storage::set_persistent(env, &crate::DataKey::Proposal(id), p);
}

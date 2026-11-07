//! Events. The first topic is the struct name in snake_case (for example `deposited`).
//! Each event carries what an indexer needs to rebuild state without reading storage.

use soroban_sdk::{contractevent, Address, BytesN};

use crate::{Config, VoteChoice};

/// `from` deposited `amount`. `deposit` and `total` are the values after the change.
#[contractevent]
pub struct Deposited {
    #[topic]
    pub account: Address,
    pub amount: i128,
    pub deposit: i128,
    pub total: i128,
}

/// `account` withdrew `amount`. `deposit` and `total` are the values after the change.
#[contractevent]
pub struct Withdrawn {
    #[topic]
    pub account: Address,
    pub amount: i128,
    pub deposit: i128,
    pub total: i128,
}

/// A proposal was created.
#[contractevent]
pub struct ProposalCreated {
    #[topic]
    pub proposal_id: u64,
    #[topic]
    pub proposer: Address,
    pub snapshot_ledger: u32,
    pub vote_start: u64,
    pub vote_end: u64,
    pub description_hash: BytesN<32>,
    pub total_power: i128,
    pub quorum: i128,
}

/// A vote was cast.
#[contractevent]
pub struct VoteCast {
    #[topic]
    pub proposal_id: u64,
    #[topic]
    pub voter: Address,
    pub choice: VoteChoice,
    pub power: i128,
}

/// A proposal was canceled by its proposer.
#[contractevent]
pub struct ProposalCanceled {
    #[topic]
    pub proposal_id: u64,
}

/// A proposal was queued; it can be executed from `eta`.
#[contractevent]
pub struct ProposalQueued {
    #[topic]
    pub proposal_id: u64,
    pub eta: u64,
}

/// A proposal was executed.
#[contractevent]
pub struct ProposalExecuted {
    #[topic]
    pub proposal_id: u64,
}

/// The settings were replaced by an executed proposal.
#[contractevent]
pub struct ConfigUpdated {
    #[topic]
    pub proposal_id: u64,
    pub config: Config,
}

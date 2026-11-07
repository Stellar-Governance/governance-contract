//! Public data types stored by the contract and exchanged with callers.

use soroban_sdk::{contracttype, Address, BytesN, String, Symbol, Val, Vec};

/// Governance settings. Times are ledger timestamps in seconds.
/// Changed only by a passed proposal carrying [`Action::UpdateConfig`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// Seconds from proposal creation until voting opens.
    pub voting_delay_secs: u64,
    /// Length of the voting window in seconds.
    pub voting_period_secs: u64,
    /// Seconds between `queue` and the earliest `execute`.
    pub timelock_secs: u64,
    /// Seconds after the timelock during which `execute` is allowed; after that the proposal is Expired.
    pub grace_period_secs: u64,
    /// Power at the snapshot ledger needed to create a proposal (0 allowed).
    pub proposal_threshold: i128,
    /// Quorum in basis points (1..=10_000) of total power at the snapshot. For + Abstain must reach it.
    pub quorum_bps: u32,
}

/// A voter's choice. The numeric values are part of the interface.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VoteChoice {
    Against = 0,
    For = 1,
    Abstain = 2,
}

/// A call the contract makes to another contract when a proposal executes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Call {
    /// Target contract. Never the governance token or this contract (`ForbiddenTarget`).
    pub contract: Address,
    /// Function name on the target.
    pub function: Symbol,
    /// Arguments, at most `MAX_ARGS`.
    pub args: Vec<Val>,
}

/// One step of a proposal.
// WAVE-ISSUE(contract-4): an `Upgrade(BytesN<32>)` action for contract upgrades plugs in here.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    /// Replace the governance settings. Affects only proposals created later.
    UpdateConfig(Config),
    /// Call another contract as the governance contract.
    Invoke(Call),
}

/// Lifecycle state of a proposal, derived from time and flags.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ProposalState {
    /// Created; voting has not opened.
    Pending = 0,
    /// Voting is open.
    Active = 1,
    /// Voting ended without passing (quorum missed, tie, or more Against).
    Defeated = 2,
    /// Voting ended and the proposal passed; waiting for `queue`.
    Succeeded = 3,
    /// Queued; executable from `eta` until `eta + grace`.
    Queued = 4,
    /// Executed. Final.
    Executed = 5,
    /// Canceled by its proposer. Final.
    Canceled = 6,
    /// Queued but not executed within the grace period. Final.
    Expired = 7,
}

/// A stored proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub proposer: Address,
    pub title: String,
    /// sha256 of the full proposal text, kept off-chain.
    pub description_hash: BytesN<32>,
    /// Where the full text can be read.
    pub uri: String,
    pub actions: Vec<Action>,
    /// Voting power is read from changes in ledgers strictly before this one.
    pub snapshot_ledger: u32,
    pub vote_start: u64,
    pub vote_end: u64,
    /// Total deposited power at the snapshot, fixed at creation.
    pub total_power: i128,
    /// Votes (For + Abstain) needed, fixed at creation.
    pub quorum: i128,
    pub for_votes: i128,
    pub against_votes: i128,
    pub abstain_votes: i128,
    /// Earliest execution time; 0 until queued.
    pub eta: u64,
    pub executed: bool,
    pub canceled: bool,
}

/// How one address voted on one proposal.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoteRecord {
    pub choice: VoteChoice,
    pub power: i128,
}

/// Voting power from `ledger` onward, until the next checkpoint.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub ledger: u32,
    pub power: i128,
}

//! Contract error codes. The numbers are part of the public interface; never renumber.

use soroban_sdk::contracterror;

/// Every way a contract call can fail.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// A config value is outside its allowed bounds.
    InvalidConfig = 1,
    /// The amount must be greater than zero.
    InvalidAmount = 2,
    /// The withdrawal is larger than the account's deposit.
    InsufficientDeposit = 3,
    /// The proposer's power at the snapshot is below the proposal threshold.
    BelowThreshold = 4,
    /// A proposal may hold at most 5 actions, and an Invoke at most 8 arguments.
    TooManyActions = 5,
    /// An Invoke action may not target the governance token or this contract.
    ForbiddenTarget = 6,
    /// The title (max 120 bytes) or URI (max 200 bytes) is too long.
    TextTooLong = 7,
    /// No proposal exists with this id.
    ProposalNotFound = 8,
    /// Voting is not open for this proposal.
    VotingClosed = 9,
    /// This address has already voted on this proposal.
    AlreadyVoted = 10,
    /// This address had no voting power at the proposal's snapshot.
    NoVotingPower = 11,
    /// The proposal is not in a state that allows this call.
    WrongState = 12,
    /// The proposal is queued but its timelock has not passed yet.
    TimelockActive = 13,
    /// The caller is not allowed to do this.
    NotAuthorized = 14,
    /// A proposal action failed while executing; nothing was changed.
    ActionFailed = 15,
    /// The token transfer failed (for example, insufficient token balance).
    TokenFailed = 16,
    /// An arithmetic result was out of range.
    Overflow = 17,
    /// The requested ledger is in the future.
    SnapshotNotPast = 18,
}

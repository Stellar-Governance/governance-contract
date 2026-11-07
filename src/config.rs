//! Config bounds. Rehearsal-friendly; revisit before any mainnet idea.

use crate::{Config, Error};

/// One day in seconds.
pub const DAY_SECS: u64 = 86_400;
/// Upper bound for every duration setting (30 days).
pub const MAX_DURATION_SECS: u64 = 30 * DAY_SECS;
/// Shortest voting period.
pub const MIN_VOTING_PERIOD_SECS: u64 = 60;
/// Shortest grace period.
pub const MIN_GRACE_PERIOD_SECS: u64 = 60;
/// 100 % in basis points.
pub const MAX_BPS: u32 = 10_000;

/// Checks every config bound. Returns `InvalidConfig` on the first violation.
pub fn validate(config: &Config) -> Result<(), Error> {
    let ok = (MIN_VOTING_PERIOD_SECS..=MAX_DURATION_SECS).contains(&config.voting_period_secs)
        && config.voting_delay_secs <= MAX_DURATION_SECS
        && config.timelock_secs <= MAX_DURATION_SECS
        && (MIN_GRACE_PERIOD_SECS..=MAX_DURATION_SECS).contains(&config.grace_period_secs)
        && (1..=MAX_BPS).contains(&config.quorum_bps)
        && config.proposal_threshold >= 0;
    if ok {
        Ok(())
    } else {
        Err(Error::InvalidConfig)
    }
}

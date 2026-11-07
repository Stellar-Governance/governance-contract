extern crate std;

use super::{default_config, panic_message, setup_with};
use crate::{Config, Governance, MAX_DURATION_SECS};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn constructor_stores_config_and_token() {
    let env = Env::default();
    let config = default_config();
    let (client, token) = setup_with(&env, &config);
    assert_eq!(client.get_config(), config);
    assert_eq!(client.token(), token);
    assert_eq!(client.proposal_count(), 0);
}

#[test]
fn constructor_accepts_bounds_at_their_limits() {
    let env = Env::default();
    let low = Config {
        voting_delay_secs: 0,
        voting_period_secs: 60,
        timelock_secs: 0,
        grace_period_secs: 60,
        proposal_threshold: 0,
        quorum_bps: 1,
    };
    let high = Config {
        voting_delay_secs: MAX_DURATION_SECS,
        voting_period_secs: MAX_DURATION_SECS,
        timelock_secs: MAX_DURATION_SECS,
        grace_period_secs: MAX_DURATION_SECS,
        proposal_threshold: i128::MAX,
        quorum_bps: 10_000,
    };
    assert_eq!(setup_with(&env, &low).0.get_config(), low);
    assert_eq!(setup_with(&env, &high).0.get_config(), high);
}

/// Each case breaks exactly one bound.
fn invalid_configs() -> std::vec::Vec<(&'static str, Config)> {
    let base = default_config();
    let with = |f: fn(&mut Config)| {
        let mut c = base.clone();
        f(&mut c);
        c
    };
    std::vec![
        ("period below 60 s", with(|c| c.voting_period_secs = 59)),
        (
            "period above 30 days",
            with(|c| c.voting_period_secs = MAX_DURATION_SECS + 1)
        ),
        (
            "delay above 30 days",
            with(|c| c.voting_delay_secs = MAX_DURATION_SECS + 1)
        ),
        (
            "timelock above 30 days",
            with(|c| c.timelock_secs = MAX_DURATION_SECS + 1)
        ),
        ("grace below 60 s", with(|c| c.grace_period_secs = 59)),
        (
            "grace above 30 days",
            with(|c| c.grace_period_secs = MAX_DURATION_SECS + 1)
        ),
        ("quorum 0 bps", with(|c| c.quorum_bps = 0)),
        ("quorum above 10000 bps", with(|c| c.quorum_bps = 10_001)),
        ("negative threshold", with(|c| c.proposal_threshold = -1)),
    ]
}

#[test]
fn validate_rejects_each_invalid_bound() {
    for (name, config) in invalid_configs() {
        assert_eq!(
            crate::config::validate(&config),
            Err(crate::Error::InvalidConfig),
            "{name}"
        );
    }
    assert_eq!(crate::config::validate(&default_config()), Ok(()));
}

#[test]
fn constructor_fails_with_invalid_config_error() {
    for (name, config) in invalid_configs() {
        let env = Env::default();
        let token = Address::generate(&env);
        let message = panic_message(std::panic::AssertUnwindSafe(|| {
            env.register(Governance, (token, config))
        }));
        let message = message.unwrap_or_else(|| std::panic!("{name}: constructor accepted it"));
        assert!(
            message.contains("Error(Contract, #1)"),
            "{name}: unexpected failure {message}"
        );
    }
}

//! Test helpers shared by the test modules.
extern crate std;

mod config;

use crate::{Config, Governance, GovernanceClient};
use soroban_sdk::{testutils::Address as _, Address, Env};
use std::string::String as StdString;

/// Rehearsal-like settings used by most tests.
pub fn default_config() -> Config {
    Config {
        voting_delay_secs: 60,
        voting_period_secs: 120,
        timelock_secs: 60,
        grace_period_secs: 86_400,
        proposal_threshold: 10,
        quorum_bps: 2_000,
    }
}

/// Registers the contract with `config` and a fresh token address.
pub fn setup_with(env: &Env, config: &Config) -> (GovernanceClient<'static>, Address) {
    let token = Address::generate(env);
    let id = env.register(Governance, (token.clone(), config.clone()));
    (GovernanceClient::new(env, &id), token)
}

/// Runs `f` and returns its panic message, or `None` if it did not panic.
pub fn panic_message<F: FnOnce() -> R + std::panic::UnwindSafe, R>(f: F) -> Option<StdString> {
    let prev = std::panic::take_hook();
    std::panic::set_hook(std::boxed::Box::new(|_| {}));
    let result = std::panic::catch_unwind(f);
    std::panic::set_hook(prev);
    match result {
        Ok(_) => None,
        Err(payload) => Some(
            payload
                .downcast_ref::<StdString>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| StdString::from(*s)))
                .unwrap_or_default(),
        ),
    }
}

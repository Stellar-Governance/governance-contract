//! Test helpers shared by the test modules.
extern crate std;

mod config;
mod deposits;
mod proposals;

use crate::{Action, Config, Error, Governance, GovernanceClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env, String, Vec,
};
use std::string::String as StdString;

/// Ledger sequence and time the tests start at.
pub const START_LEDGER: u32 = 1_000;
pub const START_TIME: u64 = 1_700_000_000;

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

/// Registers the contract with `config` and a fresh (non-contract) token address.
pub fn setup_with(env: &Env, config: &Config) -> (GovernanceClient<'static>, Address) {
    let token = Address::generate(env);
    let id = env.register(Governance, (token.clone(), config.clone()));
    (GovernanceClient::new(env, &id), token)
}

/// A governance contract bound to a real Stellar Asset Contract.
pub struct World {
    pub env: Env,
    pub gov: GovernanceClient<'static>,
    pub token: TokenClient<'static>,
    pub sac: StellarAssetClient<'static>,
}

impl World {
    /// Fresh world at `START_LEDGER` / `START_TIME`. Auth is NOT mocked; tests call
    /// `env.mock_all_auths()` or `env.mock_auths(..)` themselves.
    pub fn new(config: &Config) -> World {
        let env = Env::default();
        env.ledger().set_sequence_number(START_LEDGER);
        env.ledger().set_timestamp(START_TIME);
        let issuer = Address::generate(&env);
        let sac_contract = env.register_stellar_asset_contract_v2(issuer);
        let token_id = sac_contract.address();
        let id = env.register(Governance, (token_id.clone(), config.clone()));
        World {
            gov: GovernanceClient::new(&env, &id),
            token: TokenClient::new(&env, &token_id),
            sac: StellarAssetClient::new(&env, &token_id),
            env,
        }
    }

    /// A new account holding `amount` tokens (mints with all auths mocked).
    pub fn holder(&self, amount: i128) -> Address {
        let a = Address::generate(&self.env);
        if amount > 0 {
            self.env.mock_all_auths();
            self.sac.mint(&a, &amount);
        }
        a
    }

    /// Moves to the next ledger, `secs` seconds later.
    pub fn next_ledger(&self, secs: u64) {
        let l = self.env.ledger().get();
        self.env.ledger().set_sequence_number(l.sequence_number + 1);
        self.env.ledger().set_timestamp(l.timestamp + secs);
    }

    /// Current ledger sequence.
    pub fn ledger(&self) -> u32 {
        self.env.ledger().sequence()
    }

    /// Current ledger time.
    pub fn now(&self) -> u64 {
        self.env.ledger().timestamp()
    }

    /// Moves to the next ledger at absolute time `t`.
    pub fn at_time(&self, t: u64) {
        let l = self.env.ledger().get();
        self.env.ledger().set_sequence_number(l.sequence_number + 1);
        self.env.ledger().set_timestamp(t);
    }

    /// A holder with `amount` deposited, visible to snapshots from the next ledger on.
    pub fn voter(&self, amount: i128) -> Address {
        let a = self.holder(amount);
        self.env.mock_all_auths();
        self.gov.deposit(&a, &amount);
        a
    }

    /// The description hash used in tests.
    pub fn hash(&self) -> BytesN<32> {
        BytesN::from_array(&self.env, &[7u8; 32])
    }

    /// A Soroban string.
    pub fn text(&self, s: &str) -> String {
        String::from_str(&self.env, s)
    }

    /// Proposes `actions` as `proposer` (auths mocked) and returns the raw result.
    pub fn try_propose(&self, proposer: &Address, actions: &Vec<Action>) -> Result<u64, Error> {
        self.env.mock_all_auths();
        match self.gov.try_propose(
            proposer,
            &self.text("Raise quorum"),
            &self.hash(),
            &self.text("https://example.org/p/1"),
            actions,
        ) {
            Ok(Ok(id)) => Ok(id),
            Err(Ok(e)) => Err(e),
            other => std::panic!("unexpected result {other:?}"),
        }
    }

    /// Proposes `actions` and returns the id; panics on failure.
    pub fn propose(&self, proposer: &Address, actions: &Vec<Action>) -> u64 {
        self.try_propose(proposer, actions)
            .unwrap_or_else(|e| std::panic!("propose failed: {e:?}"))
    }

    /// A signal proposal (no actions).
    pub fn signal(&self, proposer: &Address) -> u64 {
        self.propose(proposer, &Vec::new(&self.env))
    }
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

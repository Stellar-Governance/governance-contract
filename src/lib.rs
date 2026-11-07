//! Stellar Governance contract (skeleton; the real interface arrives in task 1.1).
#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct Governance;

#[contractimpl]
impl Governance {
    /// Number of proposals created so far. The skeleton has none.
    pub fn proposal_count(_env: Env) -> u64 {
        0
    }
}

#[cfg(test)]
mod test;

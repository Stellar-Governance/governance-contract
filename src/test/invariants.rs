//! Property-style checks of the invariants listed in docs/CONTRACT.md ("Tests") over long random
//! operation sequences.
//! The generator is a fixed-seed linear congruential generator, so every run is identical.
extern crate std;

use super::{World, START_TIME};
use crate::{Config, ProposalState, VoteChoice};
use soroban_sdk::{Address, Vec};
use std::collections::{BTreeMap, BTreeSet};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // Knuth's MMIX constants.
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn config() -> Config {
    Config {
        voting_delay_secs: 10,
        voting_period_secs: 60,
        timelock_secs: 20,
        grace_period_secs: 400,
        proposal_threshold: 1,
        quorum_bps: 1_500,
    }
}

struct Model {
    seen_votes: BTreeMap<(u64, usize), (VoteChoice, i128)>,
    executed: BTreeSet<u64>,
    ids: BTreeSet<u64>,
}

fn check(w: &World, accounts: &[Address], model: &Model) {
    // 1. Sum of deposits == latest total checkpoint <= contract balance of the token.
    let sum: i128 = accounts.iter().map(|a| w.gov.deposit_of(a)).sum();

    // The latest total is visible to snapshots from the next ledger; read it via storage.
    let latest_total: i128 = w.env.as_contract(&w.gov.address, || {
        crate::checkpoints::latest(&w.env, &crate::checkpoints::Series::Total)
    });
    assert_eq!(sum, latest_total, "deposits vs total checkpoint");
    assert!(
        latest_total <= w.token.balance(&w.gov.address),
        "total vs balance"
    );

    let count = w.gov.proposal_count();
    for id in 1..=count {
        let p = w.gov.get_proposal(&id);
        // 3. Tallies never exceed the snapshot's total power.
        let cast = p.for_votes + p.against_votes + p.abstain_votes;
        assert!(
            cast <= p.total_power,
            "proposal {id}: {cast} > {}",
            p.total_power
        );
        // 2. + 6. Each vote equals the voter's power at the snapshot and never changes.
        let mut recorded = 0i128;
        for (i, a) in accounts.iter().enumerate() {
            if let Some(v) = w.gov.get_vote(&id, a) {
                let at_snapshot = w.gov.power_at(a, &p.snapshot_ledger);
                assert_eq!(v.power, at_snapshot, "vote weight vs snapshot power");
                if let Some(first) = model.seen_votes.get(&(id, i)) {
                    assert_eq!(*first, (v.choice, v.power), "a vote changed");
                }
                recorded += v.power;
            }
        }
        assert_eq!(recorded, cast, "tallies vs vote records");
        // 4. The executed flag is set exactly for the proposals the model saw executed.
        assert_eq!(p.executed, model.executed.contains(&id));
        // A canceled proposal was never executed (cancel refuses Executed, execute refuses Canceled).
        assert!(
            !(p.canceled && p.executed),
            "proposal {id} canceled and executed"
        );
    }
}

fn run(seed: u64, steps: u32, stats: &mut Stats) {
    let w = World::new(&config());
    w.env.cost_estimate().budget().reset_unlimited();
    let mut rng = Rng(seed);
    let accounts: std::vec::Vec<Address> = (0..5).map(|_| w.holder(10_000)).collect();
    let mut model = Model {
        seen_votes: BTreeMap::new(),
        executed: BTreeSet::new(),
        ids: BTreeSet::new(),
    };
    w.env.mock_all_auths();
    for _ in 0..steps {
        let who = rng.below(accounts.len() as u64) as usize;
        let a = &accounts[who];
        let count = w.gov.proposal_count();
        // Mostly act on the newest proposal so some of them run their whole lifecycle.
        let pick = match (count, rng.below(4)) {
            (0, _) => 0,
            (n, 0) => 1 + rng.below(n),
            (n, _) => n,
        };
        match rng.below(9) {
            0 | 1 => {
                let _ = w.gov.try_deposit(a, &(1 + rng.below(3_000) as i128));
            }
            2 => {
                let _ = w.gov.try_withdraw(a, &(1 + rng.below(3_000) as i128));
            }
            3 => {
                // Move tokens between identities (the double-vote attempt).
                let to = &accounts[rng.below(accounts.len() as u64) as usize];
                let bal = w.token.balance(a);
                if bal > 0 && to != a {
                    w.token
                        .transfer(a, to, &(1 + rng.below(bal as u64) as i128));
                }
            }
            4 => {
                if let Ok(Ok(id)) =
                    w.gov
                        .try_propose(a, &w.text("t"), &w.hash(), &w.text("u"), &Vec::new(&w.env))
                {
                    assert!(model.ids.insert(id), "id {id} reused");
                    assert_eq!(id, count + 1);
                }
            }
            5 | 6 if pick > 0 => {
                let choice = match rng.below(3) {
                    0 => VoteChoice::Against,
                    1 => VoteChoice::For,
                    _ => VoteChoice::Abstain,
                };
                if let Ok(Ok(power)) = w.gov.try_cast_vote(a, &pick, &choice) {
                    assert!(model
                        .seen_votes
                        .insert((pick, who), (choice, power))
                        .is_none());
                }
            }
            7 if pick > 0 => {
                // Queue in one step, execute in a later one (the timelock is 20 s).
                let before = w.gov.state(&pick);
                if before == ProposalState::Succeeded {
                    assert!(
                        w.gov.try_queue(&pick).is_ok(),
                        "queue of a Succeeded proposal"
                    );
                } else if w.gov.try_execute(&pick).is_ok() {
                    assert_eq!(before, ProposalState::Queued, "executed from {before:?}");
                    assert!(model.executed.insert(pick), "executed twice");
                } else {
                    assert!(!model.executed.contains(&pick) || before == ProposalState::Executed);
                }
            }
            8 if pick > 0 => {
                let before = w.gov.state(&pick);
                if w.gov
                    .try_cancel(&w.gov.get_proposal(&pick).proposer, &pick)
                    .is_ok()
                {
                    assert!(std::matches!(
                        before,
                        ProposalState::Pending
                            | ProposalState::Active
                            | ProposalState::Succeeded
                            | ProposalState::Queued
                    ));
                    stats.cancels += 1;
                }
            }
            _ => {}
        }
        // Advance time and ledger most steps, sometimes staying in the same ledger.
        if rng.below(4) != 0 {
            w.next_ledger(1 + rng.below(15));
        }
        check(&w, &accounts, &model);
    }
    // Every finished-but-not-executed proposal stays unexecuted forever.
    w.at_time(START_TIME + 1_000_000);
    for id in 1..=w.gov.proposal_count() {
        let state = w.gov.state(&id);
        if state != ProposalState::Executed {
            assert!(w.gov.try_execute(&id).is_err(), "late execute of {id}");
        }
    }
    check(&w, &accounts, &model);
    stats.proposals += w.gov.proposal_count();
    stats.votes += model.seen_votes.len() as u64;
    stats.executions += model.executed.len() as u64;
}

#[derive(Default)]
struct Stats {
    proposals: u64,
    votes: u64,
    executions: u64,
    cancels: u64,
}

#[test]
fn invariants_hold_over_random_sequences() {
    let mut stats = Stats::default();
    for seed in [7u64, 2026, 0xdead_beef] {
        run(seed, 150, &mut stats);
    }
    // The sequences must actually exercise the interesting paths.
    assert!(
        stats.proposals > 0 && stats.votes > 0,
        "no proposals or votes"
    );
    assert!(stats.executions > 0, "no proposal was executed");
    assert!(stats.cancels > 0, "no proposal was canceled");
}

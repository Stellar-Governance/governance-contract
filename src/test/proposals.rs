extern crate std;

use super::{default_config, World};
use crate::{
    events, proposals, Action, Call, Config, Error, Proposal, ProposalState, MAX_ACTIONS, MAX_ARGS,
    MAX_TITLE_BYTES, MAX_URI_BYTES,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    vec, Address, Event as _, IntoVal, Val, Vec,
};

fn world() -> World {
    World::new(&default_config())
}

fn invoke(w: &World, target: &Address, n_args: u32) -> Action {
    let mut args: Vec<Val> = Vec::new(&w.env);
    for i in 0..n_args {
        args.push_back(i.into_val(&w.env));
    }
    Action::Invoke(Call {
        contract: target.clone(),
        function: symbol_short!("ping"),
        args,
    })
}

#[test]
fn propose_stores_everything_and_emits_event() {
    let w = world();
    let alice = w.voter(600);
    let _bob = w.voter(400);
    w.next_ledger(5);
    let snapshot = w.ledger();
    let created = w.now();
    let new_config = Config {
        quorum_bps: 3_000,
        ..default_config()
    };
    let actions = vec![&w.env, Action::UpdateConfig(new_config)];
    let id = w.propose(&alice, &actions);
    assert_eq!(id, 1);
    let expected = events::ProposalCreated {
        proposal_id: 1,
        proposer: alice.clone(),
        snapshot_ledger: snapshot,
        vote_start: created + 60,
        vote_end: created + 180,
        description_hash: w.hash(),
        total_power: 1_000,
        quorum: 200,
    };
    assert_eq!(
        w.env.events().all().filter_by_contract(&w.gov.address),
        std::vec![expected.to_xdr(&w.env, &w.gov.address)]
    );
    let p = w.gov.get_proposal(&1);
    assert_eq!(p.proposer, alice);
    assert_eq!(p.title, w.text("Raise quorum"));
    assert_eq!(p.uri, w.text("https://example.org/p/1"));
    assert_eq!(p.actions, actions);
    assert_eq!(p.snapshot_ledger, snapshot);
    assert_eq!((p.vote_start, p.vote_end), (created + 60, created + 180));
    assert_eq!((p.total_power, p.quorum), (1_000, 200));
    assert_eq!((p.timelock_secs, p.grace_period_secs), (60, 86_400));
    assert_eq!((p.for_votes, p.against_votes, p.abstain_votes), (0, 0, 0));
    assert_eq!((p.eta, p.executed, p.canceled), (0, false, false));
    assert_eq!(w.gov.proposal_count(), 1);
    assert_eq!(w.signal(&alice), 2);
}

#[test]
fn propose_requires_the_proposers_auth() {
    let w = world();
    let alice = w.voter(600);
    w.next_ledger(5);
    let mallory = Address::generate(&w.env);
    w.env.mock_auths(&[MockAuth {
        address: &mallory,
        invoke: &MockAuthInvoke {
            contract: &w.gov.address,
            fn_name: "propose",
            args: (
                alice.clone(),
                w.text("t"),
                w.hash(),
                w.text("u"),
                Vec::<Action>::new(&w.env),
            )
                .into_val(&w.env),
            sub_invokes: &[],
        },
    }]);
    let result = w.gov.try_propose(
        &alice,
        &w.text("t"),
        &w.hash(),
        &w.text("u"),
        &Vec::new(&w.env),
    );
    assert!(result.is_err());
    assert_eq!(w.gov.proposal_count(), 0);
}

#[test]
fn below_threshold_is_rejected_and_same_ledger_deposits_do_not_count() {
    let w = world();
    let small = w.voter(9);
    w.next_ledger(5);
    assert_eq!(
        w.try_propose(&small, &Vec::new(&w.env)),
        Err(Error::BelowThreshold)
    );
    // A deposit in the proposal's own ledger is not part of its snapshot.
    let late = w.voter(1_000);
    assert_eq!(
        w.try_propose(&late, &Vec::new(&w.env)),
        Err(Error::BelowThreshold)
    );
    w.next_ledger(5);
    assert_eq!(w.try_propose(&late, &Vec::new(&w.env)), Ok(1));
}

#[test]
fn threshold_zero_lets_anyone_propose() {
    let w = World::new(&Config {
        proposal_threshold: 0,
        ..default_config()
    });
    let nobody = Address::generate(&w.env);
    assert_eq!(w.try_propose(&nobody, &Vec::new(&w.env)), Ok(1));
    let p = w.gov.get_proposal(&1);
    assert_eq!((p.total_power, p.quorum), (0, 0));
}

#[test]
fn too_many_actions_or_args_are_rejected() {
    let w = world();
    let alice = w.voter(100);
    w.next_ledger(5);
    let target = Address::generate(&w.env);
    let mut six = Vec::new(&w.env);
    for _ in 0..=MAX_ACTIONS {
        six.push_back(invoke(&w, &target, 0));
    }
    assert_eq!(w.try_propose(&alice, &six), Err(Error::TooManyActions));
    let mut five = six.clone();
    five.pop_back();
    assert_eq!(w.try_propose(&alice, &five), Ok(1));

    let nine_args = vec![&w.env, invoke(&w, &target, MAX_ARGS + 1)];
    assert_eq!(
        w.try_propose(&alice, &nine_args),
        Err(Error::TooManyActions)
    );
    let eight_args = vec![&w.env, invoke(&w, &target, MAX_ARGS)];
    assert_eq!(w.try_propose(&alice, &eight_args), Ok(2));
}

#[test]
fn long_title_or_uri_is_rejected() {
    let w = world();
    let alice = w.voter(100);
    w.next_ledger(5);
    w.env.mock_all_auths();
    let long_title = "t".repeat(MAX_TITLE_BYTES as usize + 1);
    let long_uri = "u".repeat(MAX_URI_BYTES as usize + 1);
    let ok_title = "t".repeat(MAX_TITLE_BYTES as usize);
    let ok_uri = "u".repeat(MAX_URI_BYTES as usize);
    let none = Vec::<Action>::new(&w.env);
    let try_with = |t: &str, u: &str| {
        w.gov
            .try_propose(&alice, &w.text(t), &w.hash(), &w.text(u), &none)
    };
    assert_eq!(try_with(&long_title, &ok_uri), Err(Ok(Error::TextTooLong)));
    assert_eq!(try_with(&ok_title, &long_uri), Err(Ok(Error::TextTooLong)));
    assert_eq!(try_with(&ok_title, &ok_uri), Ok(Ok(1)));
}

#[test]
fn invoke_on_the_token_or_this_contract_is_forbidden() {
    let w = world();
    let alice = w.voter(100);
    w.next_ledger(5);
    let on_token = vec![&w.env, invoke(&w, &w.token.address, 0)];
    assert_eq!(
        w.try_propose(&alice, &on_token),
        Err(Error::ForbiddenTarget)
    );
    let on_self = vec![&w.env, invoke(&w, &w.gov.address, 0)];
    assert_eq!(w.try_propose(&alice, &on_self), Err(Error::ForbiddenTarget));
    assert_eq!(w.gov.proposal_count(), 0);
}

#[test]
fn invalid_update_config_is_rejected() {
    let w = world();
    let alice = w.voter(100);
    w.next_ledger(5);
    let bad = Config {
        quorum_bps: 0,
        ..default_config()
    };
    let actions = vec![&w.env, Action::UpdateConfig(bad)];
    assert_eq!(w.try_propose(&alice, &actions), Err(Error::InvalidConfig));
}

#[test]
fn unknown_ids_are_not_found() {
    let w = world();
    assert_eq!(w.gov.try_state(&1), Err(Ok(Error::ProposalNotFound)));
    assert_eq!(w.gov.try_get_proposal(&0), Err(Ok(Error::ProposalNotFound)));
}

#[test]
fn state_moves_with_time_pending_active_defeated() {
    let w = world();
    let alice = w.voter(100);
    w.next_ledger(5);
    let id = w.signal(&alice);
    let p = w.gov.get_proposal(&id);
    assert_eq!(w.gov.state(&id), ProposalState::Pending);
    w.at_time(p.vote_start - 1);
    assert_eq!(w.gov.state(&id), ProposalState::Pending);
    w.at_time(p.vote_start);
    assert_eq!(w.gov.state(&id), ProposalState::Active);
    w.at_time(p.vote_end - 1);
    assert_eq!(w.gov.state(&id), ProposalState::Active);
    w.at_time(p.vote_end);
    assert_eq!(w.gov.state(&id), ProposalState::Defeated);
}

#[test]
fn quorum_is_a_ceiling_with_a_minimum_of_one() {
    assert_eq!(proposals::quorum(0, 2_000), Ok(0));
    assert_eq!(proposals::quorum(1_000, 2_000), Ok(200));
    assert_eq!(proposals::quorum(1_001, 2_000), Ok(201)); // 200.2 rounds up
    assert_eq!(proposals::quorum(3, 1), Ok(1)); // 0.0003 rounds up to the minimum of 1
    assert_eq!(proposals::quorum(7, 10_000), Ok(7));
    assert_eq!(proposals::quorum(i128::MAX, 2), Err(Error::Overflow));
}

/// Every branch of the derived state machine, on hand-built proposals.
#[test]
fn derived_state_covers_every_branch() {
    let w = world();
    let base = Proposal {
        proposer: Address::generate(&w.env),
        title: w.text("t"),
        description_hash: w.hash(),
        uri: w.text("u"),
        actions: Vec::new(&w.env),
        snapshot_ledger: 1,
        vote_start: 100,
        vote_end: 200,
        total_power: 1_000,
        quorum: 200,
        timelock_secs: 60,
        grace_period_secs: 1_000,
        for_votes: 0,
        against_votes: 0,
        abstain_votes: 0,
        eta: 0,
        executed: false,
        canceled: false,
    };
    let with = |f: fn(&mut Proposal)| {
        let mut p = base.clone();
        f(&mut p);
        p
    };
    use ProposalState::*;
    assert_eq!(proposals::state(&base, 99), Pending);
    assert_eq!(proposals::state(&base, 100), Active);
    assert_eq!(proposals::state(&base, 200), Defeated); // no votes
    let passing = with(|p| p.for_votes = 200);
    assert_eq!(proposals::state(&passing, 200), Succeeded);
    // Quorum counts Abstain: 150 For + 50 Abstain reaches 200.
    let with_abstain = with(|p| {
        p.for_votes = 150;
        p.abstain_votes = 50;
    });
    assert_eq!(proposals::state(&with_abstain, 200), Succeeded);
    let short_of_quorum = with(|p| {
        p.for_votes = 150;
        p.abstain_votes = 49;
    });
    assert_eq!(proposals::state(&short_of_quorum, 200), Defeated);
    let tie = with(|p| {
        p.for_votes = 300;
        p.against_votes = 300;
    });
    assert_eq!(proposals::state(&tie, 200), Defeated);
    let against = with(|p| {
        p.for_votes = 300;
        p.against_votes = 301;
    });
    assert_eq!(proposals::state(&against, 200), Defeated);
    let no_power = with(|p| {
        p.total_power = 0;
        p.quorum = 0;
        p.for_votes = 0;
    });
    assert_eq!(proposals::state(&no_power, 200), Defeated);
    let queued = with(|p| {
        p.for_votes = 200;
        p.eta = 300;
    });
    assert_eq!(proposals::state(&queued, 250), Queued);
    assert_eq!(proposals::state(&queued, 1_299), Queued);
    assert_eq!(proposals::state(&queued, 1_300), Expired);
    let executed = with(|p| {
        p.eta = 300;
        p.executed = true;
    });
    assert_eq!(proposals::state(&executed, 5_000), Executed);
    let canceled = with(|p| p.canceled = true);
    assert_eq!(proposals::state(&canceled, 0), Canceled);
    assert_eq!(proposals::state(&canceled, 5_000), Canceled);
}

extern crate std;

use super::{default_config, World};
use crate::{events, DataKey, Error, Proposal, VoteChoice, VoteRecord};
use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    Address, Event as _, IntoVal,
};

/// Voters with 600 / 300 / 100 power and an Active signal proposal by the first one.
fn active() -> (World, u64, [Address; 3]) {
    let w = World::new(&default_config());
    let a = w.voter(600);
    let b = w.voter(300);
    let c = w.voter(100);
    w.next_ledger(5);
    let id = w.signal(&a);
    w.at_time(w.gov.get_proposal(&id).vote_start);
    (w, id, [a, b, c])
}

#[test]
fn each_choice_is_counted_and_recorded() {
    let (w, id, [a, b, c]) = active();
    w.env.mock_all_auths();
    assert_eq!(w.gov.cast_vote(&a, &id, &VoteChoice::For), 600);
    assert_eq!(w.gov.cast_vote(&b, &id, &VoteChoice::Against), 300);
    assert_eq!(w.gov.cast_vote(&c, &id, &VoteChoice::Abstain), 100);
    let p = w.gov.get_proposal(&id);
    assert_eq!(
        (p.for_votes, p.against_votes, p.abstain_votes),
        (600, 300, 100)
    );
    assert_eq!(
        w.gov.get_vote(&id, &b),
        Some(VoteRecord {
            choice: VoteChoice::Against,
            power: 300
        })
    );
    assert_eq!(w.gov.get_vote(&id, &Address::generate(&w.env)), None);
}

#[test]
fn vote_emits_event() {
    let (w, id, [a, ..]) = active();
    w.env.mock_all_auths();
    w.gov.cast_vote(&a, &id, &VoteChoice::For);
    let expected = events::VoteCast {
        proposal_id: id,
        voter: a.clone(),
        choice: VoteChoice::For,
        power: 600,
    };
    assert_eq!(
        w.env.events().all().filter_by_contract(&w.gov.address),
        std::vec![expected.to_xdr(&w.env, &w.gov.address)]
    );
}

#[test]
fn vote_requires_the_voters_auth() {
    let (w, id, [a, ..]) = active();
    let mallory = Address::generate(&w.env);
    w.env.mock_auths(&[MockAuth {
        address: &mallory,
        invoke: &MockAuthInvoke {
            contract: &w.gov.address,
            fn_name: "cast_vote",
            args: (a.clone(), id, VoteChoice::For).into_val(&w.env),
            sub_invokes: &[],
        },
    }]);
    assert!(w.gov.try_cast_vote(&a, &id, &VoteChoice::For).is_err());
    assert_eq!(w.gov.get_vote(&id, &a), None);
}

#[test]
fn voting_before_start_or_after_end_is_closed() {
    let w = World::new(&default_config());
    let a = w.voter(600);
    w.next_ledger(5);
    let id = w.signal(&a);
    let p = w.gov.get_proposal(&id);
    w.env.mock_all_auths();
    assert_eq!(
        w.gov.try_cast_vote(&a, &id, &VoteChoice::For),
        Err(Ok(Error::VotingClosed))
    );
    w.at_time(p.vote_end);
    assert_eq!(
        w.gov.try_cast_vote(&a, &id, &VoteChoice::For),
        Err(Ok(Error::VotingClosed))
    );
    // The last second of the window is still open.
    let id2 = w.signal(&a);
    let p2 = w.gov.get_proposal(&id2);
    w.at_time(p2.vote_end - 1);
    assert_eq!(w.gov.cast_vote(&a, &id2, &VoteChoice::For), 600);
}

#[test]
fn double_vote_is_rejected_and_the_first_vote_stays() {
    let (w, id, [a, ..]) = active();
    w.env.mock_all_auths();
    w.gov.cast_vote(&a, &id, &VoteChoice::For);
    assert_eq!(
        w.gov.try_cast_vote(&a, &id, &VoteChoice::Against),
        Err(Ok(Error::AlreadyVoted))
    );
    let p = w.gov.get_proposal(&id);
    assert_eq!((p.for_votes, p.against_votes), (600, 0));
}

#[test]
fn zero_power_cannot_vote() {
    let (w, id, _) = active();
    let nobody = Address::generate(&w.env);
    w.env.mock_all_auths();
    assert_eq!(
        w.gov.try_cast_vote(&nobody, &id, &VoteChoice::For),
        Err(Ok(Error::NoVotingPower))
    );
}

#[test]
fn unknown_proposal_is_not_found() {
    let (w, _, [a, ..]) = active();
    w.env.mock_all_auths();
    assert_eq!(
        w.gov.try_cast_vote(&a, &99, &VoteChoice::For),
        Err(Ok(Error::ProposalNotFound))
    );
}

#[test]
fn canceled_proposal_cannot_be_voted_on() {
    let (w, id, [a, ..]) = active();
    // `cancel` arrives in task 1.5; set the flag directly to test the vote path alone.
    w.env.as_contract(&w.gov.address, || {
        let key = DataKey::Proposal(id);
        let mut p: Proposal = w.env.storage().persistent().get(&key).unwrap();
        p.canceled = true;
        w.env.storage().persistent().set(&key, &p);
    });
    w.env.mock_all_auths();
    assert_eq!(
        w.gov.try_cast_vote(&a, &id, &VoteChoice::For),
        Err(Ok(Error::VotingClosed))
    );
}

/// Invariant (PLAYBOOK §4): after a proposal exists, moving tokens to another identity gives
/// that identity no power on it, and the first voter's weight stays what it was.
#[test]
fn moved_tokens_cannot_vote_twice() {
    let (w, id, [a, ..]) = active();
    w.env.mock_all_auths();
    assert_eq!(w.gov.cast_vote(&a, &id, &VoteChoice::For), 600);
    // a withdraws everything and hands the tokens to a fresh identity, which deposits them.
    w.gov.withdraw(&a, &600);
    let sybil = Address::generate(&w.env);
    w.token.transfer(&a, &sybil, &600);
    w.gov.deposit(&sybil, &600);
    w.next_ledger(5);
    assert_eq!(w.gov.deposit_of(&sybil), 600);
    assert_eq!(
        w.gov
            .power_at(&sybil, &w.gov.get_proposal(&id).snapshot_ledger),
        0
    );
    assert_eq!(
        w.gov.try_cast_vote(&sybil, &id, &VoteChoice::For),
        Err(Ok(Error::NoVotingPower))
    );
    let p = w.gov.get_proposal(&id);
    assert_eq!(p.for_votes, 600);
    assert_eq!(w.gov.get_vote(&id, &a).map(|v| v.power), Some(600));
}

/// A voter who withdraws and re-deposits more after the snapshot still votes with the power
/// at the snapshot (PLAYBOOK §1 scenario (e), contract side).
#[test]
fn redeposit_after_snapshot_does_not_change_weight() {
    let (w, id, [_, b, _]) = active();
    w.env.mock_all_auths();
    w.gov.withdraw(&b, &300);
    w.sac.mint(&b, &200);
    w.gov.deposit(&b, &500);
    w.next_ledger(5);
    assert_eq!(w.gov.deposit_of(&b), 500);
    assert_eq!(w.gov.cast_vote(&b, &id, &VoteChoice::Against), 300);
}

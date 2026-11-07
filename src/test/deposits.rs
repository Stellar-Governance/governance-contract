extern crate std;

use super::{default_config, World};
use crate::{events, DataKey, Error};
use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    Address, Event as _, IntoVal,
};

fn world() -> World {
    World::new(&default_config())
}

#[test]
fn deposit_moves_tokens_and_raises_power() {
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    assert_eq!(w.gov.deposit(&alice, &400), 400);
    assert_eq!(w.gov.deposit_of(&alice), 400);
    assert_eq!(w.token.balance(&alice), 600);
    assert_eq!(w.token.balance(&w.gov.address), 400);
    // Same ledger: not visible to a snapshot at this ledger, visible to the next one.
    assert_eq!(w.gov.power_at(&alice, &w.ledger()), 0);
    w.next_ledger(5);
    assert_eq!(w.gov.power_at(&alice, &w.ledger()), 400);
    assert_eq!(w.gov.total_power_at(&w.ledger()), 400);
}

#[test]
fn deposit_emits_event() {
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    w.gov.deposit(&alice, &250);
    let expected = events::Deposited {
        account: alice.clone(),
        amount: 250,
        deposit: 250,
        total: 250,
    };
    assert_eq!(
        w.env.events().all().filter_by_contract(&w.gov.address),
        std::vec![expected.to_xdr(&w.env, &w.gov.address)]
    );
}

#[test]
fn deposit_requires_the_depositors_auth() {
    let w = world();
    let alice = w.holder(1_000);
    let mallory = Address::generate(&w.env);
    // Only mallory signs, so depositing from alice must fail.
    w.env.mock_auths(&[MockAuth {
        address: &mallory,
        invoke: &MockAuthInvoke {
            contract: &w.gov.address,
            fn_name: "deposit",
            args: (alice.clone(), 100i128).into_val(&w.env),
            sub_invokes: &[],
        },
    }]);
    assert!(w.gov.try_deposit(&alice, &100).is_err());
    assert_eq!(w.gov.deposit_of(&alice), 0);
    assert_eq!(w.token.balance(&alice), 1_000);
}

#[test]
fn zero_or_negative_amounts_are_rejected() {
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    assert_eq!(w.gov.try_deposit(&alice, &0), Err(Ok(Error::InvalidAmount)));
    assert_eq!(
        w.gov.try_deposit(&alice, &-5),
        Err(Ok(Error::InvalidAmount))
    );
    w.gov.deposit(&alice, &10);
    assert_eq!(
        w.gov.try_withdraw(&alice, &0),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(
        w.gov.try_withdraw(&alice, &-1),
        Err(Ok(Error::InvalidAmount))
    );
}

#[test]
fn insufficient_token_balance_maps_to_token_failed_and_changes_nothing() {
    let w = world();
    let alice = w.holder(100);
    w.env.mock_all_auths();
    assert_eq!(w.gov.try_deposit(&alice, &101), Err(Ok(Error::TokenFailed)));
    assert_eq!(w.gov.deposit_of(&alice), 0);
    assert_eq!(w.token.balance(&alice), 100);
    assert_eq!(w.token.balance(&w.gov.address), 0);
    w.next_ledger(5);
    assert_eq!(w.gov.total_power_at(&w.ledger()), 0);
}

#[test]
fn partial_and_full_withdraw() {
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    w.gov.deposit(&alice, &1_000);
    w.next_ledger(5);
    assert_eq!(w.gov.withdraw(&alice, &300), 700);
    // `events().all()` holds the events of the last call only, so check them first.
    let expected = events::Withdrawn {
        account: alice.clone(),
        amount: 300,
        deposit: 700,
        total: 700,
    };
    assert_eq!(
        w.env.events().all().filter_by_contract(&w.gov.address),
        std::vec![expected.to_xdr(&w.env, &w.gov.address)]
    );
    assert_eq!(w.token.balance(&alice), 300);
    w.next_ledger(5);
    assert_eq!(w.gov.withdraw(&alice, &700), 0);
    assert_eq!(w.token.balance(&alice), 1_000);
    assert_eq!(w.token.balance(&w.gov.address), 0);
    w.next_ledger(5);
    assert_eq!(w.gov.power_at(&alice, &w.ledger()), 0);
    assert_eq!(w.gov.total_power_at(&w.ledger()), 0);
}

#[test]
fn withdraw_above_deposit_is_rejected() {
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    w.gov.deposit(&alice, &100);
    assert_eq!(
        w.gov.try_withdraw(&alice, &101),
        Err(Ok(Error::InsufficientDeposit))
    );
    let stranger = Address::generate(&w.env);
    assert_eq!(
        w.gov.try_withdraw(&stranger, &1),
        Err(Ok(Error::InsufficientDeposit))
    );
    assert_eq!(w.gov.deposit_of(&alice), 100);
}

/// Research item 3: the contract's own `transfer` call needs no signature, because the contract
/// is the direct invoker. Only the user's auth is mocked here.
#[test]
fn withdraw_needs_only_the_users_auth() {
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    w.gov.deposit(&alice, &500);
    w.env.mock_auths(&[MockAuth {
        address: &alice,
        invoke: &MockAuthInvoke {
            contract: &w.gov.address,
            fn_name: "withdraw",
            args: (alice.clone(), 200i128).into_val(&w.env),
            sub_invokes: &[],
        },
    }]);
    assert_eq!(w.gov.withdraw(&alice, &200), 300);
    assert_eq!(w.token.balance(&alice), 700);
}

#[test]
fn power_at_many_ledgers() {
    let w = world();
    let alice = w.holder(1_000);
    let bob = w.holder(1_000);
    w.env.mock_all_auths();
    let before_any = w.ledger();
    w.next_ledger(5);
    let l1 = w.ledger();
    w.gov.deposit(&alice, &100);
    w.next_ledger(5);
    w.next_ledger(5);
    let l2 = w.ledger();
    w.gov.deposit(&alice, &50);
    w.gov.deposit(&bob, &30);
    w.next_ledger(5);
    let l3 = w.ledger();
    w.gov.withdraw(&alice, &120);
    w.next_ledger(5);
    let now = w.ledger();

    // Before the first deposit.
    assert_eq!(w.gov.power_at(&alice, &before_any), 0);
    // Exactly at a checkpoint ledger: changes in that ledger are not counted.
    assert_eq!(w.gov.power_at(&alice, &l1), 0);
    assert_eq!(w.gov.power_at(&alice, &(l1 + 1)), 100);
    // Between checkpoints.
    assert_eq!(w.gov.power_at(&alice, &(l2 - 1)), 100);
    assert_eq!(w.gov.power_at(&alice, &l2), 100);
    assert_eq!(w.gov.power_at(&alice, &l3), 150);
    // The current ledger sees everything before it.
    assert_eq!(w.gov.power_at(&alice, &now), 30);
    assert_eq!(w.gov.power_at(&bob, &now), 30);
    assert_eq!(w.gov.total_power_at(&l2), 100);
    assert_eq!(w.gov.total_power_at(&l3), 180);
    assert_eq!(w.gov.total_power_at(&now), 60);
}

#[test]
fn several_changes_in_one_ledger_make_one_checkpoint() {
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    w.gov.deposit(&alice, &100);
    w.gov.deposit(&alice, &100);
    w.gov.withdraw(&alice, &50);
    let count: u32 = w.env.as_contract(&w.gov.address, || {
        w.env
            .storage()
            .persistent()
            .get(&DataKey::CkptCount(alice.clone()))
            .unwrap_or(0)
    });
    assert_eq!(count, 1);
    w.next_ledger(5);
    assert_eq!(w.gov.power_at(&alice, &w.ledger()), 150);
    assert_eq!(w.gov.total_power_at(&w.ledger()), 150);
}

#[test]
fn future_ledgers_are_rejected() {
    let w = world();
    let alice = w.holder(0);
    let future = w.ledger() + 1;
    assert_eq!(
        w.gov.try_power_at(&alice, &future),
        Err(Ok(Error::SnapshotNotPast))
    );
    assert_eq!(
        w.gov.try_total_power_at(&future),
        Err(Ok(Error::SnapshotNotPast))
    );
}

#[test]
fn writes_extend_ttl_to_the_maximum() {
    use soroban_sdk::testutils::storage::Persistent as _;
    let w = world();
    let alice = w.holder(1_000);
    w.env.mock_all_auths();
    w.gov.deposit(&alice, &10);
    let (ttl, max) = w.env.as_contract(&w.gov.address, || {
        (
            w.env
                .storage()
                .persistent()
                .get_ttl(&DataKey::Deposit(alice.clone())),
            w.env.storage().max_ttl(),
        )
    });
    assert_eq!(ttl, max);
}

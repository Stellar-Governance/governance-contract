//! cancel / queue / execute, and what `Invoke` actions can and cannot do.
extern crate std;

use super::{default_config, World};
use crate::{events, Action, Call, Config, Error, GovernanceClient, ProposalState, VoteChoice};
use soroban_sdk::{
    contract, contracterror, contractimpl, symbol_short,
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    vec, Address, Env, Event as _, IntoVal, Symbol, Val, Vec,
};

/// A contract for `Invoke` actions to call.
#[contract]
pub struct Target;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TargetError {
    /// Always returned by `fail`.
    Nope = 1,
}

#[contractimpl]
impl Target {
    /// Stores `x` so a test can see the call happened.
    pub fn ping(env: Env, x: u32) -> u32 {
        env.storage().instance().set(&symbol_short!("x"), &x);
        x
    }

    pub fn last(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("x"))
            .unwrap_or(0)
    }

    pub fn fail(_env: Env) -> Result<(), TargetError> {
        Err(TargetError::Nope)
    }

    /// Calls back into the governance contract while it is executing.
    pub fn reenter(env: Env, gov: Address, id: u64) {
        GovernanceClient::new(&env, &gov).execute(&id);
    }
}

fn call(w: &World, target: &Address, function: &str, args: Vec<Val>) -> Action {
    Action::Invoke(Call {
        contract: target.clone(),
        function: Symbol::new(&w.env, function),
        args,
    })
}

struct Setup {
    w: World,
    a: Address,
    b: Address,
}

/// Voters a (600) and b (400); nothing proposed yet.
fn setup() -> Setup {
    let w = World::new(&default_config());
    let a = w.voter(600);
    let b = w.voter(400);
    w.next_ledger(5);
    Setup { w, a, b }
}

impl Setup {
    /// Creates a proposal by `a`, votes For with a, and moves past the end: Succeeded.
    fn succeeded(&self, actions: &Vec<Action>) -> u64 {
        let id = self.w.propose(&self.a, actions);
        let p = self.w.gov.get_proposal(&id);
        self.w.at_time(p.vote_start);
        self.w.env.mock_all_auths();
        self.w.gov.cast_vote(&self.a, &id, &VoteChoice::For);
        self.w.at_time(p.vote_end);
        assert_eq!(self.w.gov.state(&id), ProposalState::Succeeded);
        id
    }

    /// Succeeded and queued; returns (id, eta).
    fn queued(&self, actions: &Vec<Action>) -> (u64, u64) {
        let id = self.succeeded(actions);
        let eta = self.w.gov.queue(&id);
        (id, eta)
    }

    fn cancel(&self, who: &Address, id: u64) -> Result<(), Error> {
        self.w.env.mock_all_auths();
        match self.w.gov.try_cancel(who, &id) {
            Ok(Ok(())) => Ok(()),
            Err(Ok(e)) => Err(e),
            other => std::panic!("unexpected {other:?}"),
        }
    }
}

fn none(w: &World) -> Vec<Action> {
    Vec::new(&w.env)
}

// ---------- cancel ----------

#[test]
fn proposer_can_cancel_in_each_allowed_state() {
    let s = setup();
    let w = &s.w;
    // Pending
    let pending = w.signal(&s.a);
    assert_eq!(s.cancel(&s.a, pending), Ok(()));
    assert_eq!(w.gov.state(&pending), ProposalState::Canceled);
    // Active
    let active = w.signal(&s.a);
    w.at_time(w.gov.get_proposal(&active).vote_start);
    assert_eq!(s.cancel(&s.a, active), Ok(()));
    assert_eq!(w.gov.state(&active), ProposalState::Canceled);
    // Succeeded
    let succeeded = s.succeeded(&none(w));
    assert_eq!(s.cancel(&s.a, succeeded), Ok(()));
    assert_eq!(w.gov.state(&succeeded), ProposalState::Canceled);
    // Queued, and a canceled proposal can never be executed.
    let (queued, eta) = s.queued(&none(w));
    assert_eq!(s.cancel(&s.a, queued), Ok(()));
    w.at_time(eta);
    assert_eq!(w.gov.try_execute(&queued), Err(Ok(Error::WrongState)));
    assert_eq!(w.gov.state(&queued), ProposalState::Canceled);
}

#[test]
fn cancel_emits_event() {
    let s = setup();
    let id = s.w.signal(&s.a);
    s.w.env.mock_all_auths();
    s.w.gov.cancel(&s.a, &id);
    let expected = events::ProposalCanceled { proposal_id: id };
    assert_eq!(
        s.w.env.events().all().filter_by_contract(&s.w.gov.address),
        std::vec![expected.to_xdr(&s.w.env, &s.w.gov.address)]
    );
}

#[test]
fn stranger_cannot_cancel() {
    let s = setup();
    let id = s.w.signal(&s.a);
    assert_eq!(s.cancel(&s.b, id), Err(Error::NotAuthorized));
    assert_eq!(s.w.gov.state(&id), ProposalState::Pending);
}

#[test]
fn cancel_needs_the_callers_auth() {
    let s = setup();
    let id = s.w.signal(&s.a);
    // b signs, but the call claims to be from a.
    s.w.env.mock_auths(&[MockAuth {
        address: &s.b,
        invoke: &MockAuthInvoke {
            contract: &s.w.gov.address,
            fn_name: "cancel",
            args: (s.a.clone(), id).into_val(&s.w.env),
            sub_invokes: &[],
        },
    }]);
    assert!(s.w.gov.try_cancel(&s.a, &id).is_err());
    assert_eq!(s.w.gov.state(&id), ProposalState::Pending);
}

#[test]
fn cancel_after_execution_or_defeat_is_wrong_state() {
    let s = setup();
    let w = &s.w;
    let (id, eta) = s.queued(&none(w));
    w.at_time(eta);
    w.gov.execute(&id);
    assert_eq!(s.cancel(&s.a, id), Err(Error::WrongState));
    let defeated = w.signal(&s.a);
    w.at_time(w.gov.get_proposal(&defeated).vote_end);
    assert_eq!(s.cancel(&s.a, defeated), Err(Error::WrongState));
    assert_eq!(s.cancel(&s.a, 99), Err(Error::ProposalNotFound));
}

// ---------- queue ----------

#[test]
fn queue_sets_eta_and_emits_event() {
    let s = setup();
    let id = s.succeeded(&none(&s.w));
    let now = s.w.now();
    assert_eq!(s.w.gov.queue(&id), now + 60);
    let expected = events::ProposalQueued {
        proposal_id: id,
        eta: now + 60,
    };
    assert_eq!(
        s.w.env.events().all().filter_by_contract(&s.w.gov.address),
        std::vec![expected.to_xdr(&s.w.env, &s.w.gov.address)]
    );
    assert_eq!(s.w.gov.state(&id), ProposalState::Queued);
    assert_eq!(s.w.gov.get_proposal(&id).eta, now + 60);
    // Queuing twice is refused.
    assert_eq!(s.w.gov.try_queue(&id), Err(Ok(Error::WrongState)));
}

#[test]
fn queue_refuses_pending_active_and_defeated() {
    let s = setup();
    let w = &s.w;
    let id = w.signal(&s.a);
    assert_eq!(w.gov.try_queue(&id), Err(Ok(Error::WrongState))); // Pending
    w.at_time(w.gov.get_proposal(&id).vote_start);
    assert_eq!(w.gov.try_queue(&id), Err(Ok(Error::WrongState))); // Active
    w.at_time(w.gov.get_proposal(&id).vote_end);
    assert_eq!(w.gov.state(&id), ProposalState::Defeated);
    assert_eq!(w.gov.try_queue(&id), Err(Ok(Error::WrongState)));
    assert_eq!(w.gov.try_queue(&99), Err(Ok(Error::ProposalNotFound)));
}

/// Runs a vote with the given (For, Against, Abstain) voters and returns the final state.
fn outcome(quorum_bps: u32, votes: &[(i128, VoteChoice)]) -> ProposalState {
    let w = World::new(&Config {
        quorum_bps,
        ..default_config()
    });
    let voters: std::vec::Vec<(Address, VoteChoice)> =
        votes.iter().map(|(amt, c)| (w.voter(*amt), *c)).collect();
    let _bystander = w.voter(1_000);
    w.next_ledger(5);
    let id = w.signal(&voters[0].0);
    let p = w.gov.get_proposal(&id);
    w.at_time(p.vote_start);
    w.env.mock_all_auths();
    for (v, c) in &voters {
        w.gov.cast_vote(v, &id, c);
    }
    w.at_time(p.vote_end);
    let state = w.gov.state(&id);
    if state == ProposalState::Defeated {
        assert_eq!(w.gov.try_queue(&id), Err(Ok(Error::WrongState)));
    }
    state
}

#[test]
fn quorum_missed_tie_and_majority_against_are_defeated() {
    use VoteChoice::*;
    // Total power is 1_000 + the voters. Quorum 50 %.
    // Quorum missed: 300 For of 1_300 total, needs 650.
    assert_eq!(outcome(5_000, &[(300, For)]), ProposalState::Defeated);
    // Tie: 500 For and 500 Against with a 1 % quorum, so only the tie rule decides.
    assert_eq!(
        outcome(100, &[(500, For), (500, Against)]),
        ProposalState::Defeated
    );
    // Majority Against.
    assert_eq!(
        outcome(100, &[(400, For), (600, Against)]),
        ProposalState::Defeated
    );
    // Abstain counts towards quorum but not towards the majority: 700 For + 600 Abstain of
    // 2_300 total reaches 50 % (1_150).
    assert_eq!(
        outcome(5_000, &[(700, For), (600, Abstain)]),
        ProposalState::Succeeded
    );
}

// ---------- execute ----------

#[test]
fn execute_respects_the_timelock_window() {
    let s = setup();
    let w = &s.w;
    let (id, eta) = s.queued(&none(w));
    w.at_time(eta - 1);
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::TimelockActive)));
    w.at_time(eta);
    w.gov.execute(&id);
    let expected = events::ProposalExecuted { proposal_id: id };
    assert_eq!(
        w.env.events().all().filter_by_contract(&w.gov.address),
        std::vec![expected.to_xdr(&w.env, &w.gov.address)]
    );
    assert_eq!(w.gov.state(&id), ProposalState::Executed);
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::WrongState)));
}

#[test]
fn execute_after_grace_is_expired() {
    let s = setup();
    let w = &s.w;
    let (id, eta) = s.queued(&none(w));
    let grace = w.gov.get_proposal(&id).grace_period_secs;
    w.at_time(eta + grace - 1);
    assert_eq!(w.gov.state(&id), ProposalState::Queued);
    w.at_time(eta + grace);
    assert_eq!(w.gov.state(&id), ProposalState::Expired);
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::WrongState)));
    assert_eq!(s.cancel(&s.a, id), Err(Error::WrongState));
}

#[test]
fn execute_refuses_unqueued_or_unknown_proposals() {
    let s = setup();
    let w = &s.w;
    let id = s.succeeded(&none(w));
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::WrongState)));
    assert_eq!(w.gov.try_execute(&99), Err(Ok(Error::ProposalNotFound)));
}

#[test]
fn update_config_applies_only_to_later_proposals() {
    let s = setup();
    let w = &s.w;
    let new_config = Config {
        voting_delay_secs: 0,
        voting_period_secs: 300,
        timelock_secs: 600,
        grace_period_secs: 3_600,
        proposal_threshold: 50,
        quorum_bps: 5_000,
    };
    // An older proposal, created before the change.
    let older = w.signal(&s.a);
    let (id, eta) = s.queued(&vec![&w.env, Action::UpdateConfig(new_config.clone())]);
    w.at_time(eta);
    w.gov.execute(&id);
    let updated = events::ConfigUpdated {
        proposal_id: id,
        config: new_config.clone(),
    };
    let executed = events::ProposalExecuted { proposal_id: id };
    assert_eq!(
        w.env.events().all().filter_by_contract(&w.gov.address),
        std::vec![
            updated.to_xdr(&w.env, &w.gov.address),
            executed.to_xdr(&w.env, &w.gov.address)
        ]
    );
    assert_eq!(w.gov.get_config(), new_config);
    let old = w.gov.get_proposal(&older);
    assert_eq!((old.timelock_secs, old.grace_period_secs), (60, 86_400));
    assert_eq!(old.quorum, 200);
    let later = w.signal(&s.a);
    let p = w.gov.get_proposal(&later);
    assert_eq!((p.timelock_secs, p.grace_period_secs), (600, 3_600));
    assert_eq!(p.quorum, 500);
    assert_eq!(p.vote_end - p.vote_start, 300);
}

/// Research item 3, PLAYBOOK §1 (b): the contract transfers a token it holds, through `Invoke`,
/// with no signature from anyone (the contract is the direct invoker of the transfer).
#[test]
fn invoke_transfers_a_treasury_token() {
    let s = setup();
    let w = &s.w;
    let treasury_sac = w
        .env
        .register_stellar_asset_contract_v2(Address::generate(&w.env));
    let treasury = TokenClient::new(&w.env, &treasury_sac.address());
    w.env.mock_all_auths();
    StellarAssetClient::new(&w.env, &treasury.address).mint(&w.gov.address, &1_000);
    let recipient = Address::generate(&w.env);
    let args: Vec<Val> = vec![
        &w.env,
        w.gov.address.into_val(&w.env),
        recipient.into_val(&w.env),
        250i128.into_val(&w.env),
    ];
    let (id, eta) = s.queued(&vec![&w.env, call(w, &treasury.address, "transfer", args)]);
    w.at_time(eta);
    // No auths mocked at all: anyone can execute, and the transfer needs no signature.
    w.env.mock_auths(&[]);
    w.gov.execute(&id);
    assert_eq!(treasury.balance(&recipient), 250);
    assert_eq!(treasury.balance(&w.gov.address), 750);
}

#[test]
fn invoke_runs_actions_in_order() {
    let s = setup();
    let w = &s.w;
    let target = w.env.register(Target, ());
    let t = TargetClient::new(&w.env, &target);
    let actions = vec![
        &w.env,
        call(w, &target, "ping", vec![&w.env, 1u32.into_val(&w.env)]),
        call(w, &target, "ping", vec![&w.env, 2u32.into_val(&w.env)]),
    ];
    let (id, eta) = s.queued(&actions);
    w.at_time(eta);
    w.gov.execute(&id);
    assert_eq!(t.last(), 2);
}

#[test]
fn a_failing_invoke_reverts_everything() {
    let s = setup();
    let w = &s.w;
    let target = w.env.register(Target, ());
    let t = TargetClient::new(&w.env, &target);
    let new_config = Config {
        quorum_bps: 9_000,
        ..default_config()
    };
    let actions = vec![
        &w.env,
        Action::UpdateConfig(new_config),
        call(w, &target, "ping", vec![&w.env, 5u32.into_val(&w.env)]),
        call(w, &target, "fail", Vec::new(&w.env)),
    ];
    let (id, eta) = s.queued(&actions);
    w.at_time(eta);
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::ActionFailed)));
    assert_eq!(w.gov.state(&id), ProposalState::Queued);
    assert!(!w.gov.get_proposal(&id).executed);
    assert_eq!(w.gov.get_config(), default_config());
    assert_eq!(t.last(), 0);
}

#[test]
fn a_call_to_a_missing_function_fails_the_action() {
    let s = setup();
    let w = &s.w;
    let target = w.env.register(Target, ());
    let (id, eta) = s.queued(&vec![
        &w.env,
        call(w, &target, "nothing_here", Vec::new(&w.env)),
    ]);
    w.at_time(eta);
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::ActionFailed)));
    assert_eq!(w.gov.state(&id), ProposalState::Queued);
}

/// Decision D3 follow-up: a target that calls back into the governance contract during
/// `execute` makes the action fail and changes nothing. This test does not tell which guard
/// stopped it: the host refusing contract re-entry, or the inner `execute` seeing the proposal
/// already marked executed. Either is enough.
#[test]
fn reentry_from_a_target_fails_the_action() {
    let s = setup();
    let w = &s.w;
    let target = w.env.register(Target, ());
    let args: Vec<Val> = vec![
        &w.env,
        w.gov.address.into_val(&w.env),
        1u64.into_val(&w.env),
    ];
    let (id, eta) = s.queued(&vec![&w.env, call(w, &target, "reenter", args)]);
    assert_eq!(id, 1);
    w.at_time(eta);
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::ActionFailed)));
    assert_eq!(w.gov.state(&id), ProposalState::Queued);
}

#[test]
fn update_config_that_is_now_invalid_fails_at_execution() {
    // Validation happens at propose time, so an invalid config cannot be stored through the
    // public interface; write one directly to check the re-validation in `execute`.
    let s = setup();
    let w = &s.w;
    let (id, eta) = s.queued(&vec![&w.env, Action::UpdateConfig(default_config())]);
    w.env.as_contract(&w.gov.address, || {
        let key = crate::DataKey::Proposal(id);
        let mut p: crate::Proposal = w.env.storage().persistent().get(&key).unwrap();
        p.actions = vec![
            &w.env,
            Action::UpdateConfig(Config {
                quorum_bps: 0,
                ..default_config()
            }),
        ];
        w.env.storage().persistent().set(&key, &p);
    });
    w.at_time(eta);
    assert_eq!(w.gov.try_execute(&id), Err(Ok(Error::ActionFailed)));
    assert_eq!(w.gov.get_config(), default_config());
}

#[test]
fn execute_is_permissionless() {
    let s = setup();
    let w = &s.w;
    let (id, eta) = s.queued(&none(w));
    w.at_time(eta);
    w.env.mock_auths(&[]);
    w.gov.execute(&id);
    assert_eq!(w.gov.state(&id), ProposalState::Executed);
}

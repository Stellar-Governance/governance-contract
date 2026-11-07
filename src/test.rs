extern crate std;

use crate::{Governance, GovernanceClient};
use soroban_sdk::Env;

#[test]
fn skeleton_has_no_proposals() {
    let env = Env::default();
    let id = env.register(Governance, ());
    let client = GovernanceClient::new(&env, &id);
    assert_eq!(client.proposal_count(), 0);
}

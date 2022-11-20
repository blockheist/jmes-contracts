#![cfg(test)]

use cosmwasm_std::{Addr, Decimal};
use cw4::Member;
use cw_multi_test::App;
use cw_utils::{Duration, MsgInstantiateContractResponse, Threshold};
use dao_members::multitest::contract::DaoMembersContract;
use dao_multisig::multitest::contract::DaoMultisigContract;
use jmes::{msg::Voter, test_utils::get_attribute};

use crate::{
    msg::{DaosResponse, GetIdentityByNameResponse, GetIdentityByOwnerResponse},
    state::Identity,
};

use super::contract::IdentityserviceContract;

#[test]
fn register_dao() {
    let _owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let mut app = App::default();

    let dao_members_code_id = DaoMembersContract::store_code(&mut app);
    let dao_multisig_code_id = DaoMultisigContract::store_code(&mut app);

    let identityservice_code_id = IdentityserviceContract::store_code(&mut app);
    let identityservice_contract = IdentityserviceContract::instantiate(
        &mut app,
        identityservice_code_id,
        &user1,
        "identityservice",
        Addr::unchecked("governance"),
        dao_members_code_id,
        dao_multisig_code_id,
    )
    .unwrap();

    // Register a new DAO
    identityservice_contract
        .register_dao(
            &mut app,
            &user1,
            // "my_dao".to_string(),
            vec![
                Member {
                    addr: user1.clone().into(),
                    weight: 1,
                },
                Member {
                    addr: user2.clone().into(),
                    weight: 1,
                },
            ],
            "my_dao".to_string(),
            Decimal::percent(51),
            Duration::Time(300),
        )
        .unwrap();

    let daos_response = identityservice_contract.query_daos(&mut app, None, None, None);
    assert_eq!(
        daos_response,
        Ok(DaosResponse {
            daos: vec![(1, Addr::unchecked("contract2"))]
        })
    );

    let identity_by_owner_response = identityservice_contract
        .query_get_identity_by_owner(&mut app, Addr::unchecked("contract2").into());
    assert_eq!(
        identity_by_owner_response,
        Ok(GetIdentityByOwnerResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("contract2").into(),
                name: "my_dao".to_string(),
                id_type: crate::state::IdType::Dao
            })
        })
    );

    let identity_by_name_response =
        identityservice_contract.query_get_identity_by_name(&mut app, "my_dao".to_string());
    assert_eq!(
        identity_by_name_response,
        Ok(GetIdentityByNameResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("contract2").into(),
                name: "my_dao".to_string(),
                id_type: crate::state::IdType::Dao
            })
        })
    );
}

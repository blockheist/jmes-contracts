#![cfg(test)]

use cosmwasm_std::{Addr, Decimal};
use cw4::Member;
use cw_multi_test::App;
use cw_utils::Duration;
use dao_members::multitest::contract::DaoMembersContract;
use dao_multisig::multitest::contract::DaoMultisigContract;
use serde::de::IntoDeserializer;

use crate::{
    msg::{DaosResponse, GetIdentityByNameResponse, GetIdentityByOwnerResponse},
    state::Identity,
};

use super::contract::IdentityserviceContract;

#[test]
fn register_user() {
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
        &user1.clone(),
        "identityservice",
        dao_members_code_id,
        dao_multisig_code_id,
        Addr::unchecked("governance"),
    )
    .unwrap();

    // Register a new User
    identityservice_contract
        .register_user(&mut app, &user1.clone(), "user1_name".into())
        .unwrap();

    let identity_by_owner_response =
        identityservice_contract.query_get_identity_by_owner(&mut app, user1.clone().into());
    assert_eq!(
        identity_by_owner_response,
        Ok(GetIdentityByOwnerResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );

    let identity_by_name_response =
        identityservice_contract.query_get_identity_by_name(&mut app, "user1_name".to_string());
    assert_eq!(
        identity_by_name_response,
        Ok(GetIdentityByNameResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );
}
#[test]
fn register_user_with_same_name_fails() {
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
        dao_members_code_id,
        dao_multisig_code_id,
        Addr::unchecked("governance"),
    )
    .unwrap();

    // Register a new User
    identityservice_contract
        .register_user(&mut app, &user1, "user1_name".into())
        .unwrap();

    let identity_by_owner_response =
        identityservice_contract.query_get_identity_by_owner(&mut app, user1.clone().into());
    assert_eq!(
        identity_by_owner_response,
        Ok(GetIdentityByOwnerResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );

    let identity_by_name_response =
        identityservice_contract.query_get_identity_by_name(&mut app, "user1_name".to_string());
    assert_eq!(
        identity_by_name_response,
        Ok(GetIdentityByNameResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );

    // Register the existing User a 2nd time, this should fail
    let err = identityservice_contract
        .register_user(&mut app, &user2, "user1_name".into())
        .unwrap_err();
    println!("\n\n err {:?}", err);
    assert_eq!(
        err,
        crate::error::ContractError::NameTaken {
            name: "user1_name".into()
        }
    );
}
#[test]
fn register_user_with_same_wallet_fails() {
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
        dao_members_code_id,
        dao_multisig_code_id,
        Addr::unchecked("governance"),
    )
    .unwrap();

    // Register a new User
    identityservice_contract
        .register_user(&mut app, &user1, "user1_name".into())
        .unwrap();

    let identity_by_owner_response =
        identityservice_contract.query_get_identity_by_owner(&mut app, user1.clone().into());
    assert_eq!(
        identity_by_owner_response,
        Ok(GetIdentityByOwnerResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );

    let identity_by_name_response =
        identityservice_contract.query_get_identity_by_name(&mut app, "user1_name".to_string());
    assert_eq!(
        identity_by_name_response,
        Ok(GetIdentityByNameResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );

    // Register a new User with the same wallet, this should fail
    let err = identityservice_contract
        .register_user(&mut app, &user1.clone(), "user1_name_different".into())
        .unwrap_err();
    println!("\n\n err {:?}", err);
    assert_eq!(err, crate::error::ContractError::AlreadyRegistered {});
}
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
        dao_members_code_id,
        dao_multisig_code_id,
        Addr::unchecked("governance"),
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
                    weight: 26,
                },
                Member {
                    addr: user2.clone().into(),
                    weight: 26,
                },
            ],
            "my_dao".to_string(),
            51u64,
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
#[test]
fn register_dao_with_existing_dao_name_fails() {
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
        dao_members_code_id,
        dao_multisig_code_id,
        Addr::unchecked("governance"),
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
                    weight: 26,
                },
                Member {
                    addr: user2.clone().into(),
                    weight: 26,
                },
            ],
            "my_dao".to_string(),
            51u64,
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

    // Register a existing DAO name
    let err = identityservice_contract
        .register_dao(
            &mut app,
            &user1,
            // "my_dao".to_string(),
            vec![
                Member {
                    addr: user1.clone().into(),
                    weight: 26,
                },
                Member {
                    addr: user2.clone().into(),
                    weight: 26,
                },
            ],
            "my_dao".to_string(),
            51u64,
            Duration::Time(300),
        )
        .unwrap_err();

    assert_eq!(
        err,
        crate::error::ContractError::NameTaken {
            name: "my_dao".into()
        }
    );
}
#[test]
fn register_dao_with_existing_user_name_fails() {
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
        dao_members_code_id,
        dao_multisig_code_id,
        Addr::unchecked("governance"),
    )
    .unwrap();
    // Register a new User
    identityservice_contract
        .register_user(&mut app, &user1, "user1_name".into())
        .unwrap();

    let identity_by_owner_response =
        identityservice_contract.query_get_identity_by_owner(&mut app, user1.clone().into());
    assert_eq!(
        identity_by_owner_response,
        Ok(GetIdentityByOwnerResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );

    let identity_by_name_response =
        identityservice_contract.query_get_identity_by_name(&mut app, "user1_name".to_string());
    assert_eq!(
        identity_by_name_response,
        Ok(GetIdentityByNameResponse {
            identity: Some(Identity {
                owner: Addr::unchecked("user1").into(),
                name: "user1_name".to_string(),
                id_type: crate::state::IdType::User
            })
        })
    );
    // Register a new DAO
    let err = identityservice_contract
        .register_dao(
            &mut app,
            &user1,
            // "my_dao".to_string(),
            vec![
                Member {
                    addr: user1.clone().into(),
                    weight: 26,
                },
                Member {
                    addr: user2.clone().into(),
                    weight: 26,
                },
            ],
            "user1_name".to_string(),
            51u64,
            Duration::Time(300),
        )
        .unwrap_err();
    println!("\n\n err {:?}", err);

    assert_eq!(
        err,
        crate::error::ContractError::NameTaken {
            name: "user1_name".into()
        }
    );
}

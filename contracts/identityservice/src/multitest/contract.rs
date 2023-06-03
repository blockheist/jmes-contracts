use cosmwasm_std::{Addr, Decimal, StdResult};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use cw_utils::Duration;

use crate::contract::{execute, instantiate, query, reply};
use crate::msg::{
    DaosResponse, ExecuteMsg, GetIdentityByNameResponse, GetIdentityByOwnerResponse,
    InstantiateMsg, Ordering, QueryMsg, RegisterDaoMsg,
};
use crate::ContractError;

#[derive(Debug, Clone)]
pub struct IdentityserviceContract(Addr);

impl IdentityserviceContract {
    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn store_code(app: &mut App) -> u64 {
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
        app.store_code(Box::new(contract))
    }

    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        code_id: u64,
        sender: &Addr,
        label: &str,

        governance_addr: Addr,
        dao_members_code_id: u64,
        dao_multisig_code_id: u64,
        goveranance_addr: Addr,
    ) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                owner: governance_addr.clone(),
                dao_members_code_id,
                dao_multisig_code_id,
                governance_addr,
            },
            &[],
            label,
            None,
        )
        .map(IdentityserviceContract)
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn register_user(
        &self,
        app: &mut App,
        sender: &Addr,
        name: String,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::RegisterUser { name },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }

    #[track_caller]
    pub fn register_dao(
        &self,
        app: &mut App,
        sender: &Addr,
        members: Vec<cw4::Member>,
        dao_name: String,
        threshold_percentage: Decimal,
        max_voting_period: Duration,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::RegisterDao(RegisterDaoMsg {
                members,
                dao_name,
                max_voting_period,
                threshold_percentage,
            }),
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }

    #[track_caller]
    pub fn query_daos(
        &self,
        app: &mut App,
        start_after: Option<u64>,
        limit: Option<u32>,
        order: Option<Ordering>,
    ) -> StdResult<DaosResponse> {
        app.wrap().query_wasm_smart(
            self.0.clone(),
            &QueryMsg::Daos {
                limit,
                order,
                start_after,
            },
        )
    }

    #[track_caller]
    pub fn query_get_identity_by_owner(
        &self,
        app: &mut App,
        owner: String,
    ) -> StdResult<GetIdentityByOwnerResponse> {
        app.wrap()
            .query_wasm_smart(self.0.clone(), &QueryMsg::GetIdentityByOwner { owner })
    }

    #[track_caller]
    pub fn query_get_identity_by_name(
        &self,
        app: &mut App,
        name: String,
    ) -> StdResult<GetIdentityByNameResponse> {
        app.wrap()
            .query_wasm_smart(self.0.clone(), &QueryMsg::GetIdentityByName { name })
    }
}

impl From<IdentityserviceContract> for Addr {
    fn from(contract: IdentityserviceContract) -> Self {
        contract.0
    }
}

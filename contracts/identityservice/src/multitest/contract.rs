use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use cw_utils::{Duration, Threshold};
use jmes::msg::{DaoInstantiateMsg, Voter};

use crate::contract::{execute, instantiate, query, reply};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::ContractError;

#[derive(Debug)]
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
        dao_code_id: u64,
    ) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                owner: governance_addr,
                dao_code_id,
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

        dao_name: String,
        voters: Vec<Voter>,
        threshold: Threshold,
        max_voting_period: Duration,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::RegisterDao(DaoInstantiateMsg {
                dao_name,
                voters,
                threshold,
                max_voting_period,
            }),
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }
}

impl From<IdentityserviceContract> for Addr {
    fn from(contract: IdentityserviceContract) -> Self {
        contract.0
    }
}

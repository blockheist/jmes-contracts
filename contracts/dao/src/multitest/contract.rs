use cosmwasm_std::{Addr, CosmosMsg, StdResult};
use cw3::Vote;
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use cw_utils::{Duration, Expiration, Threshold};

use crate::contract::{execute, instantiate, query};
use crate::msg::ExecuteMsg;
use crate::ContractError;
// use crate::error::ContractError;
use jmes::msg::DaoInstantiateMsg as InstantiateMsg;
use jmes::msg::Voter;

#[derive(Debug)]
pub struct DaoContract(Addr);

impl DaoContract {
    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn store_code(app: &mut App) -> u64 {
        let contract = ContractWrapper::new(execute, instantiate, query);
        app.store_code(Box::new(contract))
    }

    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        code_id: u64,
        sender: &Addr,
        label: &str,

        dao_name: String,
        voters: Vec<Voter>,
        threshold: Threshold,
        max_voting_period: Duration,
    ) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                dao_name,
                voters,
                threshold,
                max_voting_period,
            },
            &[],
            label,
            None,
        )
        .map(DaoContract)
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn propose(
        app: &mut App,
        sender: &Addr,

        dao_contract: &String,
        title: String,
        description: String,
        msgs: Vec<CosmosMsg>,
        latest: Option<Expiration>,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            Addr::unchecked(dao_contract),
            &ExecuteMsg::Propose {
                title,
                description,
                msgs,
                latest,
            },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn vote(
        app: &mut App,
        sender: &Addr,

        dao_contract: &String,
        proposal_id: u64,
        vote: Vote,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            Addr::unchecked(dao_contract),
            &ExecuteMsg::Vote { proposal_id, vote },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn execute(
        app: &mut App,
        sender: &Addr,

        dao_contract: &String,
        proposal_id: u64,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            Addr::unchecked(dao_contract),
            &ExecuteMsg::Execute { proposal_id },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
    }
}

impl From<DaoContract> for Addr {
    fn from(contract: DaoContract) -> Self {
        contract.0
    }
}

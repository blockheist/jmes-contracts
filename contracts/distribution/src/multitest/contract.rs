use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
// use cw_utils::{Duration, Threshold};

use crate::contract::{execute, instantiate, query};
use crate::ContractError;
// use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};

#[derive(Debug, Clone)]
pub struct DistributionContract(Addr);

impl DistributionContract {
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

        owner: Addr,
        identityservice_contract: Addr,
    ) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                owner,
                identityservice_contract,
            },
            &[],
            label,
            None,
        )
        .map(DistributionContract)
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn claim(
        &self,
        app: &mut App,
        sender: &Addr,

        grant_id: u64,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::Claim { grant_id },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }
}

impl From<DistributionContract> for Addr {
    fn from(contract: DistributionContract) -> Self {
        contract.0
    }
}

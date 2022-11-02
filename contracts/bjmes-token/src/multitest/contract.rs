use cosmwasm_std::{Addr, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};

use crate::contract::{execute, instantiate, query};
use crate::ContractError;
// use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};

#[derive(Debug, Clone)]
pub struct BjmesTokenContract(Addr);

impl BjmesTokenContract {
    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn store_code(app: &mut App) -> u64 {
        let contract = ContractWrapper::new(execute, instantiate, query);
        app.store_code(Box::new(contract))
    }

    #[track_caller]
    pub fn instantiate(app: &mut App, code_id: u64, sender: &Addr, label: &str) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                name: "bonded JMES".to_string(),
                symbol: "bjmes".to_string(),
                decimals: 10,
                initial_balances: vec![],
                marketing: None,
                mint: None,
            },
            &[],
            label,
            None,
        )
        .map(BjmesTokenContract)
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn mint(
        &self,
        app: &mut App,
        sender: &Addr,

        recipient: String,
        amount: Uint128,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::Mint { recipient, amount },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }
}

impl From<BjmesTokenContract> for Addr {
    fn from(contract: BjmesTokenContract) -> Self {
        contract.0
    }
}

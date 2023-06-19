use cosmwasm_std::{Addr, StdResult};
use cw4::Member;
use cw_multi_test::{App, ContractWrapper, Executor};
use cw_utils::Duration;

use crate::contract::{execute, instantiate, query};
use crate::msg::InstantiateMsg;

#[derive(Debug)]
pub struct DaoMembersContract(Addr);

impl DaoMembersContract {
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

        members: Vec<Member>,
        dao_name: String,
        max_voting_period: Duration,
        threshold_percentage: u64,
        governance_addr: Addr,
    ) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                members,
                dao_name,
                max_voting_period,
                threshold_percentage,
                governance_addr,
            },
            &[],
            label,
            None,
        )
        .map(DaoMembersContract)
        .map_err(|err| err.downcast().unwrap())
    }
}

impl From<DaoMembersContract> for Addr {
    fn from(contract: DaoMembersContract) -> Self {
        contract.0
    }
}

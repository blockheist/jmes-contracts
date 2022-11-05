use cosmwasm_std::{coins, Addr, Binary, CosmosMsg, StdResult, WasmMsg};
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

    #[track_caller]
    pub fn gov_proposal_helper(
        app: &mut App,
        my_dao: Addr,
        gov_contract: &Addr,
        user1: Addr,
        user2: Addr,
        proposal_msg: StdResult<Binary>,
        proposal_deposit: u128,
    ) -> Result<AppResponse, ContractError> {
        let my_dao_addr = my_dao.to_string();
        // Wrap gov proposal msg so we can attach it to the dao proposal
        let wasm_msg = WasmMsg::Execute {
            contract_addr: gov_contract.to_string(),
            msg: proposal_msg.unwrap(),
            funds: coins(proposal_deposit, "uluna"),
        };

        let submit_dao_proposal_result = DaoContract::propose(
            app,
            &user1,
            &my_dao_addr,
            "Dao Proposal".into(),
            "Wraps Governance Proposal".into(),
            vec![CosmosMsg::Wasm(wasm_msg)],
            None,
        );
        let proposal_id = submit_dao_proposal_result
            .unwrap()
            .events
            .iter()
            .find(|e| e.ty == "wasm")
            .unwrap()
            .attributes
            .iter()
            .find(|attr| attr.key == "proposal_id")
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap();

        println!("\n\n fished proposal_id {:?}", proposal_id);
        // User1 already voted automatically
        // User2 votes yes to pass the proposal
        let dao_vote2_result =
            DaoContract::vote(app, &user2, &my_dao_addr, proposal_id, cw3::Vote::Yes);
        println!("\n\n dao_vote2_result {:?}", dao_vote2_result);

        let dao_execute_result = DaoContract::execute(app, &user1, &my_dao_addr, proposal_id);
        println!("\n\n dao_execute_result {:?}", dao_execute_result);

        // Test after proposal execution the deposit is sent to the governance contract
        assert_eq!(
            app.wrap()
                .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
                .unwrap(),
            vec![]
        );
        dao_execute_result
    }
}

impl From<DaoContract> for Addr {
    fn from(contract: DaoContract) -> Self {
        contract.0
    }
}

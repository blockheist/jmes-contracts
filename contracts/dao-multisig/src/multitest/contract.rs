use cosmwasm_std::{coins, from_binary, Addr, Binary, CosmosMsg, StdResult, Uint128, WasmMsg};
use cw3::Vote;
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use cw_utils::{Duration, Expiration, Threshold};

use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, InstantiateMsg, ProposeResponse};
use crate::ContractError;

// Address for burning the proposal fee
const BURN_ADDRESS: &str = "jmes1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqf5laz2";

#[derive(Debug)]
pub struct DaoMultisigContract(Addr);

impl DaoMultisigContract {
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

        group_addr: String,
        threshold: Threshold,
        max_voting_period: Duration,
        dao_name: String,
        // who is able to execute passed proposals
        // None means that anyone can execute
        executor: Option<crate::state::Executor>,
    ) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                group_addr,
                threshold,
                max_voting_period,
                executor,
                dao_name,
            },
            &[],
            label,
            None,
        )
        .map(DaoMultisigContract)
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
        proposal_deposit: u128,
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
            &coins(proposal_deposit, "ujmes"), // attach the proposal fee to be burned, this is sent from the user to the DAO address so it can be forwarded to the gov contract
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
        proposal_msg: Binary,
        proposal_deposit: u128,
    ) -> Result<AppResponse, ContractError> {
        let my_dao_addr = my_dao.to_string();

        // Wrap gov proposal msg so we can attach it to the dao proposal
        let wasm_msg = WasmMsg::Execute {
            contract_addr: gov_contract.into(),
            msg: proposal_msg,
            funds: coins(proposal_deposit, "ujmes"), // attach the proposal fee to be burned, this is sent from the dao addr to the gov contract
        };

        let dao_propose_response = DaoMultisigContract::propose(
            app,
            &user1,
            &my_dao_addr,
            "Dao Proposal".into(),
            "Wraps Governance Proposal".into(),
            vec![CosmosMsg::Wasm(wasm_msg)],
            None,
            proposal_deposit,
        );

        let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response?.data.unwrap())
            .unwrap()
            .proposal_id;

        println!("\n\n proposal_id {:?}", proposal_id);

        // User1 already voted automatically
        // println!("user2 {:#?}", &user2);
        // User2 votes yes to pass the proposal
        let dao_vote2_result =
            DaoMultisigContract::vote(app, &user2, &my_dao_addr, proposal_id, cw3::Vote::Yes);
        println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

        let dao_execute_result =
            DaoMultisigContract::execute(app, &user1, &my_dao_addr, proposal_id);
        println!("\n\n dao_execute_result {:?}", dao_execute_result);

        // Test after proposal execution the proposal fee has left the DAO wallet
        assert_eq!(
            app.wrap()
                .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
                .unwrap(),
            vec![]
        );
        // Test that the governance contract burned the proposal fee
        assert_eq!(
            app.wrap()
                .query_all_balances(Addr::unchecked(gov_contract.clone()))
                .unwrap(),
            vec![]
        );
        // Test that the burn address received the proposal fee
        assert_eq!(
            app.wrap()
                .query_all_balances(Addr::unchecked(BURN_ADDRESS.to_string()))
                .unwrap(),
            coins(proposal_deposit, "ujmes")
        );
        dao_execute_result
    }
}

impl From<DaoMultisigContract> for Addr {
    fn from(contract: DaoMultisigContract) -> Self {
        contract.0
    }
}

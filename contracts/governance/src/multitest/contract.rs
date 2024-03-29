use cosmwasm_std::{Addr, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};

use crate::error::ContractError;
// use crate::error::ContractError;
use crate::msg::{CoreSlot, ExecuteMsg, InstantiateMsg, PeriodInfoResponse, ProposalResponse};
use jmes::msg::GovernanceCoreSlotsResponse as CoreSlotsResponse;
use jmes::msg::GovernanceQueryMsg as QueryMsg;

use crate::state::VoteOption;
use crate::{execute, instantiate, query};

#[derive(Debug, Clone)]
pub struct GovernanceContract(Addr);

impl GovernanceContract {
    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn store_code(app: &mut App) -> u64 {
        let contract = ContractWrapper::new(execute, instantiate, query);
        app.store_code(Box::new(contract))
    }

    #[track_caller]
    pub fn instantiate<'a>(
        app: &mut App,
        code_id: u64,
        sender: &Addr,
        label: &str,

        owner: String,
        proposal_required_deposit: Uint128,
        proposal_required_percentage: u64,
        period_start_epoch: u64,
        posting_period_length: u64,
        voting_period_length: u64,
    ) -> StdResult<Self> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                owner,
                proposal_required_deposit,
                proposal_required_percentage,
                period_start_epoch,
                posting_period_length,
                voting_period_length,
            },
            &[],
            label,
            None,
        )
        .map(GovernanceContract)
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn set_core_slot(
        &self,
        app: &mut App,
        sender: &Addr,

        proposal_id: u64,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::SetCoreSlot { proposal_id },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn resign_core_slot(
        &self,
        app: &mut App,
        sender: &Addr,

        slot: CoreSlot,
        note: String,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::ResignCoreSlot { slot, note },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
    }

    #[track_caller]
    pub fn set_contract(
        &self,
        app: &mut App,
        sender: &Addr,

        art_dealer: String,
        identityservice: String,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::SetContract {
                art_dealer,
                identityservice,
            },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }

    #[track_caller]
    pub fn vote(
        &self,
        app: &mut App,
        sender: &Addr,

        id: u64,
        vote: VoteOption,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::Vote { id, vote },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }

    #[track_caller]
    pub fn conclude(
        &self,
        app: &mut App,
        sender: &Addr,

        id: u64,
    ) -> Result<AppResponse, ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecuteMsg::Conclude { id },
            &[],
        )
        .map_err(|err| err.downcast().unwrap())
        // .map(|_| ())
    }

    #[track_caller]
    pub fn query_period_info(&self, app: &mut App) -> StdResult<PeriodInfoResponse> {
        app.wrap()
            .query_wasm_smart(self.0.clone(), &QueryMsg::PeriodInfo {})
    }

    #[track_caller]
    pub fn query_proposal(&self, app: &mut App, id: u64) -> StdResult<ProposalResponse> {
        app.wrap()
            .query_wasm_smart(self.0.clone(), &QueryMsg::Proposal { id })
    }

    #[track_caller]
    pub fn query_core_slots(&self, app: &mut App) -> StdResult<CoreSlotsResponse> {
        app.wrap()
            .query_wasm_smart(self.0.clone(), &QueryMsg::CoreSlots {})
    }
}

impl From<GovernanceContract> for Addr {
    fn from(contract: GovernanceContract) -> Self {
        contract.0
    }
}

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_utils::{Duration, Threshold};

#[cw_serde]
pub struct DaoInstantiateMsg {
    pub dao_name: String,
    pub voters: Vec<Voter>,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Voter {
    pub addr: String,
    pub weight: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalQueryStatus {
    Active,
    SuccessConcluded,
    ExpiredConcluded,
}

impl ProposalQueryStatus {
    pub fn to_string(&self) -> String {
        match &self {
            ProposalQueryStatus::Active => "active".to_string(),
            ProposalQueryStatus::SuccessConcluded => "success_concluded".to_string(),
            ProposalQueryStatus::ExpiredConcluded => "expired_concluded".to_string(),
        }
    }
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceQueryMsg {
    Config {},
    PeriodInfo {},
    Proposal {
        id: u64,
    },
    Proposals {
        status: ProposalQueryStatus,
        start: Option<u64>,
        limit: Option<u32>,
    },
    CoreSlots {},
    WinningGrants {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GovernanceCoreSlotsResponse {
    pub brand: Option<SlotVoteResult>,
    pub creative: Option<SlotVoteResult>,
    pub core_tech: Option<SlotVoteResult>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SlotVoteResult {
    pub dao: Addr,
    pub yes_ratio: Decimal,
    pub proposal_voting_end: u64,
    pub proposal_funding_end: u64,
    pub proposal_id: u64,
}

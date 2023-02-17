use std::fmt;

use cosmwasm_std::{Addr, CosmosMsg, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Funding, ProposalStatus, ProposalType, SlotVoteResult, VoteOption};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub owner: String,
    pub bjmes_token_addr: String,
    pub artist_curator_addr: Option<String>,
    pub proposal_required_deposit: Uint128,
    // Required percentage for a proposal to pass, e.g. 51
    pub proposal_required_percentage: u64,
    // Epoch when the 1st posting period starts, e.g. 1660000000
    pub period_start_epoch: u64,
    // Length in seconds of the posting period, e.g.  606864 for ~ 1 Week (year/52)
    pub posting_period_length: u64,
    // Length in seconds of the posting period, e.g.  606864 for ~ 1 Week (year/52)
    pub voting_period_length: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Receive a message of type [`Cw20ReceiveMsg`]
    Propose(ProposalMsg),
    Vote {
        id: u64,
        vote: VoteOption,
    },
    Conclude {
        id: u64,
    },
    SetContract {
        artist_curator: String,
        identityservice: String,
    },
    SetCoreSlot {
        proposal_id: u64,
    },
    UnsetCoreSlot {
        proposal_id: u64,
    },
    ResignCoreSlot {
        slot: CoreSlot,
        note: String, // Can be used to explain why the dao is resigning, is only added as an attribute to the events
    },
    // RemoveFeature { feature: Feature },

    // RequestCoreSlot { core_slot: CoreSlot },

    // RemoveCoreSlot { core_slot: CoreSlot },
    // BurnArtistNft {},
    // BurnArtNft {}
}

/// This structure stores the parameters for the different proposal types
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalMsg {
    TextProposal {
        title: String,
        description: String,
        funding: Option<Funding>,
    },
    RequestFeature {
        title: String,
        description: String,
        funding: Option<Funding>,
        feature: Feature,
    },
    Improvement {
        title: String,
        description: String,
        funding: Option<Funding>,
        msgs: Vec<CosmosMsg>,
    },
    CoreSlot {
        title: String,
        description: String,
        funding: Option<Funding>,
        slot: CoreSlot,
    },
    RevokeCoreSlot {
        title: String,
        description: String,
        funding: Option<Funding>,
        revoke_slot: RevokeCoreSlot,
    },
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AddGrantMsg {
    pub add_grant: AddGrant,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AddGrant {
    pub dao: Addr,
    pub duration: u64,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    ArtistCurator { approved: u64, duration: u64 },
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoreSlot {
    Brand {},
    Creative {},
    CoreTech {},
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RevokeCoreSlot {
    pub slot: CoreSlot,
    pub dao: String,
}

impl fmt::Display for CoreSlot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CoreSlot::Brand {} => write!(f, "brand"),
            CoreSlot::Creative {} => write!(f, "creative"),
            CoreSlot::CoreTech {} => write!(f, "core_tech"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PeriodInfo {},
    Proposal {
        id: u64,
    },
    Proposals {
        start: Option<u64>,
        limit: Option<u32>,
    },
    CoreSlots {},
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalPeriod {
    Posting,
    Voting,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PeriodInfoResponse {
    pub current_block: u64,
    pub current_period: ProposalPeriod,
    pub current_time_in_cycle: u64,
    pub current_posting_start: u64,
    pub current_voting_start: u64,
    pub current_voting_end: u64,
    pub next_posting_start: u64,
    pub next_voting_start: u64,
    pub posting_period_length: u64,
    pub voting_period_length: u64,
    pub cycle_length: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoreSlotsResponse {
    pub brand: Option<SlotVoteResult>,
    pub creative: Option<SlotVoteResult>,
    pub core_tech: Option<SlotVoteResult>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ProposalResponse {
    pub id: u64,
    pub dao: Addr,
    pub title: String,
    pub description: String,
    pub prop_type: ProposalType,
    pub coins_yes: Uint128,
    pub coins_no: Uint128,
    pub yes_voters: Vec<Addr>,
    pub no_voters: Vec<Addr>,
    pub deposit_amount: Uint128,
    pub start_block: u64,
    pub posting_start: u64,
    pub voting_start: u64,
    pub voting_end: u64,
    pub concluded: bool,
    pub status: ProposalStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ProposalsResponse {
    pub proposal_count: u64,
    pub proposals: Vec<ProposalResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub bjmes_token_addr: Addr,
    pub artist_curator_addr: Option<Addr>,
    pub proposal_required_deposit: Uint128,
    // Required percentage for a proposal to pass, e.g. 51
    pub proposal_required_percentage: u64,
    // Epoch when the 1st posting period starts, e.g. 1660000000
    pub period_start_epoch: u64,
    // Length in seconds of the posting period, e.g.  606864 for ~ 1 Week (year/52)
    pub posting_period_length: u64,
    // Length in seconds of the posting period, e.g.  606864 for ~ 1 Week (year/52)
    pub voting_period_length: u64,
}

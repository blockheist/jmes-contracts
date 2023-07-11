use std::fmt;

use cosmwasm_std::{Addr, CosmosMsg, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Funding, ProposalStatus, ProposalType, VoteOption, WinningGrant};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub owner: String,
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
pub struct MigrateMsg {}

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
        art_dealer: String,
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
        funding: Funding,
        feature: Feature,
    },
    Improvement {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg>,
    },
    CoreSlot {
        title: String,
        description: String,
        funding: Funding,
        slot: CoreSlot,
    },
    RevokeProposal {
        title: String,
        description: String,
        revoke_proposal_id: u64,
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
    ArtDealer { approved: u64 },
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoreSlot {
    Brand {},
    Creative {},
    CoreTech {},
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
pub struct WinningGrantsResponse {
    pub winning_grants: Vec<WinningGrant>,
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
    pub funding: Option<Funding>,
    pub concluded_at_height: Option<u64>,
    pub status: ProposalStatus,
    pub msgs: Option<Vec<CosmosMsg>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ProposalsResponse {
    pub proposal_count: u64,
    pub proposals: Vec<ProposalResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub art_dealer_addr: Option<Addr>,
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

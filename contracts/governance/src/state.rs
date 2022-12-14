use crate::{
    error::ContractError,
    msg::{CoreSlot, Feature, RevokeCoreSlot},
};
use cosmwasm_std::{Addr, CosmosMsg, Decimal, Env, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Proposal validation attributes
const MIN_TITLE_LENGTH: usize = 4;
const MAX_TITLE_LENGTH: usize = 64;
const MIN_DESC_LENGTH: usize = 4;
const MAX_DESC_LENGTH: usize = 1024;

/// Special characters that are allowed in proposal text
const SAFE_TEXT_CHARS: &str = "!&?#()*+'-./\"";

pub const CONFIG: Item<Config> = Item::new("config");

pub const CORE_SLOTS: Item<CoreSlots> = Item::new("core_slots");

pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");

pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposals");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SlotVoteResult {
    pub dao: Addr,
    pub yes_ratio: Decimal,
    pub proposal_voting_end: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoreSlots {
    pub brand: Option<SlotVoteResult>,
    pub creative: Option<SlotVoteResult>,
    pub core_tech: Option<SlotVoteResult>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub owner: Option<Addr>,
    pub bjmes_token_addr: Addr,
    pub distribution_addr: Option<Addr>,
    pub artist_curator_addr: Option<Addr>,
    pub identityservice_addr: Option<Addr>,
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
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Proposal {
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
    pub msgs: Option<Vec<CosmosMsg>>,
}

impl Proposal {
    pub fn next_id(store: &mut dyn Storage) -> StdResult<u64> {
        let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
        PROPOSAL_COUNT.save(store, &id)?;
        Ok(id)
    }

    pub fn status(&self, env: Env, proposal_required_percentage: u64) -> ProposalStatus {
        let mut status = ProposalStatus::Posted;

        if env.block.time.seconds() > self.voting_start {
            status = ProposalStatus::Voting;
        }

        if env.block.time.seconds() > self.voting_end {
            let coins_yes = self.coins_yes;
            let coins_no = self.coins_no;
            let coins_total = coins_yes + coins_no;

            let mut yes_ratio: Decimal = Decimal::zero();

            if !coins_total.is_zero() {
                yes_ratio = Decimal::from_ratio(coins_yes, coins_total);
            }

            let required_yes_ratio = Decimal::from_ratio(proposal_required_percentage, 100u64);

            status = if yes_ratio >= required_yes_ratio {
                if self.concluded {
                    ProposalStatus::SuccessConcluded
                } else {
                    ProposalStatus::Success
                }
            } else {
                if self.concluded {
                    ProposalStatus::ExpiredConcluded
                } else {
                    ProposalStatus::Expired
                }
            };
        }
        status
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        // Title validation
        if self.title.len() < MIN_TITLE_LENGTH {
            return Err(ContractError::ProposalNotValid {
                error: "Title too short!".into(),
            });
        }
        if self.title.len() > MAX_TITLE_LENGTH {
            return Err(ContractError::ProposalNotValid {
                error: "Title too long!".into(),
            });
        }
        if !self.title.chars().all(|c| {
            c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || SAFE_TEXT_CHARS.contains(c)
        }) {
            return Err(ContractError::ProposalNotValid {
                error: "Title is not in alphanumeric format!".into(),
            });
        }

        // Description validation
        if self.description.len() < MIN_DESC_LENGTH {
            return Err(ContractError::ProposalNotValid {
                error: "Description too short!".into(),
            });
        }
        if self.description.len() > MAX_DESC_LENGTH {
            return Err(ContractError::ProposalNotValid {
                error: "Description too long!".into(),
            });
        }
        if !self.description.chars().all(|c| {
            c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || SAFE_TEXT_CHARS.contains(c)
        }) {
            return Err(ContractError::ProposalNotValid {
                error: "Description is not in alphanumeric format".into(),
            });
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Posted,
    Voting,
    Success,
    Expired,
    SuccessConcluded,
    ExpiredConcluded,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalType {
    Text {},
    FeatureRequest(Feature),
    Funding {},
    Improvement {},
    CoreSlot(CoreSlot),
    RevokeCoreSlot(RevokeCoreSlot),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoteOption {
    Yes,
    No,
}

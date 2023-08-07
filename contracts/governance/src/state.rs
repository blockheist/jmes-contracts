use crate::{
    error::ContractError,
    msg::{CoreSlot, Feature},
};
use cosmwasm_std::{Addr, CosmosMsg, Decimal, Env, QuerierWrapper, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

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

pub struct ProposalIndexes<'a> {
    // pk goes to second tuple element
    pub status: MultiIndex<'a, String, Proposal, String>,
}

impl<'a> IndexList<Proposal> for ProposalIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Proposal>> + '_> {
        let v: Vec<&dyn Index<Proposal>> = vec![&self.status];
        Box::new(v.into_iter())
    }
}

pub fn proposals<'a>() -> IndexedMap<'a, String, Proposal, ProposalIndexes<'a>> {
    let indexes = ProposalIndexes {
        status: MultiIndex::new(
            |_pk: &[u8], d: &Proposal| match d.clone().concluded_status {
                Some(status) => match status {
                    ProposalStatus::SuccessConcluded => "success_concluded".to_string(),
                    ProposalStatus::ExpiredConcluded => "expired_concluded".to_string(),
                    _ => "active".to_string(),
                },
                None => "active".to_string(),
            },
            "proposals",
            "proposals__status",
        ),
    };
    IndexedMap::new("proposals", indexes)
}

// This is an item of type vec that gets updated on every conclude and old grants are deleted
pub const WINNING_GRANTS: Item<Vec<WinningGrant>> = Item::new("winning_grants");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct WinningGrant {
    pub dao: Addr,
    pub amount: Uint128,
    pub expire_at_height: u64,
    pub yes_ratio: Decimal,
    pub proposal_id: u64,
    pub max_cap: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoreSlots {
    pub brand: Option<jmes::msg::SlotVoteResult>,
    pub creative: Option<jmes::msg::SlotVoteResult>,
    pub core_tech: Option<jmes::msg::SlotVoteResult>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub owner: Option<Addr>,
    pub art_dealer_addr: Option<Addr>,
    pub identityservice_addr: Option<Addr>,
    pub proposal_required_deposit: Uint128,
    // Required net yes vote percentage required for a proposal to pass, e.g. 10
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
    pub concluded_at_height: Option<u64>,
    pub concluded_status: Option<ProposalStatus>,
    pub concluded_coins_total: Option<Uint128>,
    pub funding: Option<Funding>,
    pub msgs: Option<Vec<CosmosMsg>>,
}

impl Proposal {
    pub fn next_id(store: &mut dyn Storage) -> StdResult<u64> {
        let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
        PROPOSAL_COUNT.save(store, &id)?;
        Ok(id)
    }

    pub fn update_coins_total(&mut self, &querier: &QuerierWrapper) {
        self.concluded_coins_total = Some(self.query_coins_total(&querier));
    }

    pub fn query_coins_total(&self, &querier: &QuerierWrapper) -> Uint128 {
        if self.concluded_coins_total.is_some() {
            return self.concluded_coins_total.unwrap();
        } else {
            querier
                .query_supply("bujmes")
                .unwrap()
                .amount
                .checked_sub(Uint128::new(100_000_000_000_000)) // FIXME: remove this hack
                .unwrap()
        }
    }

    pub fn current_status(
        &self,
        &querier: &QuerierWrapper,
        env: Env,
        proposal_required_percentage: u64,
        is_concluded: bool,
    ) -> ProposalStatus {
        let mut status = ProposalStatus::Posted;

        if env.block.time.seconds() > self.voting_start {
            status = ProposalStatus::Voting;
        }

        if env.block.time.seconds() > self.voting_end {
            let coins_yes = self.coins_yes;
            let coins_no = self.coins_no;

            let coins_net_yes = coins_yes.checked_sub(coins_no).unwrap_or_default();

            let coins_total = self.query_coins_total(&querier);
            let mut yes_ratio: Decimal = Decimal::zero();

            if !coins_total.is_zero() {
                yes_ratio = Decimal::from_ratio(coins_net_yes, coins_total);
            }

            let required_yes_ratio = Decimal::from_ratio(proposal_required_percentage, 100u64);

            status = if yes_ratio >= required_yes_ratio {
                if is_concluded {
                    ProposalStatus::SuccessConcluded
                } else {
                    ProposalStatus::Success
                }
            } else {
                if is_concluded {
                    ProposalStatus::ExpiredConcluded
                } else {
                    ProposalStatus::Expired
                }
            };
        }
        status
    }

    pub fn set_concluded_status(
        &mut self,
        &querier: &QuerierWrapper,
        env: Env,
        proposal_required_percentage: u64,
    ) {
        self.concluded_status =
            Some(self.current_status(&querier, env, proposal_required_percentage, true));
    }

    pub fn query_status(
        &self,
        &querier: &QuerierWrapper,
        env: Env,
        proposal_required_percentage: u64,
    ) -> ProposalStatus {
        // If the proposal is concluded, return the final static status
        if self.concluded_status.is_some() {
            return self.concluded_status.clone().unwrap();
        } else {
            // Otherwise, return the current status based on updating cycle and coin data
            self.current_status(&querier, env, proposal_required_percentage, false)
        }
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
    Improvement {},
    CoreSlot(CoreSlot),
    RevokeProposal(u64),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoteOption {
    Yes,
    No,
}

// Funding is an optional add-on to a proposal
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Funding {
    pub amount: Uint128,
    pub duration_in_blocks: u64,
}

use cosmwasm_std::{Addr, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Grant;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub identityservice_contract: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddGrant {
        dao: Addr,
        duration: u64,
        amount: Uint128,
    },
    // RevokeGrant {
    //     dao: Addr,
    // },
    Claim {
        grant_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Grant {
        grant_id: u64,
    },
    Grants {
        dao: Option<Addr>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub identityservice_contract: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct GrantResponse {
    pub grant_id: u64,
    pub dao: Addr,
    pub amount_approved: Uint128,
    pub amount_remaining: Uint128,
    pub started: Timestamp,
    pub expires: Timestamp,
    pub claimable_amount: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GrantsResponse {
    pub grants: Vec<Grant>,
}

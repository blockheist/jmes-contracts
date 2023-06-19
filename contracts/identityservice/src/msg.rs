use cosmwasm_std::Addr;
use cw4::Member;
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Identity;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Voter {
    pub addr: String,
    pub weight: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub dao_members_code_id: u64,
    pub dao_multisig_code_id: u64,
    pub governance_addr: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RegisterDaoMsg {
    pub members: Vec<Member>,
    pub dao_name: String,
    pub threshold_percentage: u64,
    pub max_voting_period: Duration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterUser { name: String },
    RegisterDao(RegisterDaoMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ordering {
    Ascending,
    Descending,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetIdentityByOwner {
        owner: String,
    },
    GetIdentityByName {
        name: String,
    },
    Daos {
        start_after: Option<u64>,
        limit: Option<u32>,
        order: Option<Ordering>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetIdentityByOwnerResponse {
    pub identity: Option<Identity>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetIdentityByNameResponse {
    pub identity: Option<Identity>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DaosResponse {
    pub daos: Vec<(u64, Addr)>,
}

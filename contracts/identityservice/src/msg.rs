use cosmwasm_std::Addr;
use jmes::msg::DaoInstantiateMsg;
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
    pub dao_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterUser { name: String },
    RegisterDao(DaoInstantiateMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetIdentityByOwner { owner: String },
    GetIdentityByName { name: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct IdentityResponse {
    pub identity: Option<Identity>,
}

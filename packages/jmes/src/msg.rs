use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_utils::{Duration, Threshold};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
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

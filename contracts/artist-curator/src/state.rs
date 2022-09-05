use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub identityservice_contract: Addr,
    pub art_nft_address: Option<Addr>,
    pub art_nft_name: String,
    pub art_nft_symbol: String,
    pub artist_nft_address: Option<Addr>,
    pub artist_nft_name: String,
    pub artist_nft_symbol: String,
    pub artist_nft_total_tokens_minted: u32, // Sets the incrementing token_id
    pub artist_nft_circulating_supply: u32,  // minted - burned
}

pub const CONFIG: Item<Config> = Item::new("config");

// we cast a ballot with our chosen vote and a given weight
// stored under the key that voted
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    pub approved: u64,
    pub minted: u64,
    pub burned: u64,
    pub expires: Timestamp,
}

pub const APPROVALS: Map<&Addr, Approval> = Map::new("approvals");

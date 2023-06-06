use cosmwasm_std::Addr;
use cw721_metadata_onchain::Metadata;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub identityservice_contract: Addr,
    pub art_nft_name: String,
    pub art_nft_symbol: String,
    pub art_nft_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    MintArt {
        token_id: String,
        /// The owner of the newly minter NFT
        owner: String,
        /// Universal resource identifier for this NFT
        /// Should point to a JSON file that conforms to the ERC721
        /// Metadata JSON Schema
        token_uri: Option<String>,
        // see: https://docs.opensea.io/docs/metadata-standards
        metadata: Option<Metadata>,
    },
    ApproveDealer {
        dao: Addr,
        approved: u64,
        // Time in seconds
        duration: u64,
    },
    RevokeDealer {
        dao: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub identityservice_contract: Addr,
    pub art_nft_address: Option<Addr>,
    pub art_nft_name: String,
    pub art_nft_symbol: String,
}

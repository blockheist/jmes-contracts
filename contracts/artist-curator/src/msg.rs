use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub identityservice_contract: Addr,
    pub artist_nft_name: String,
    pub artist_nft_symbol: String,
    pub artist_nft_code_id: u64,
    pub art_nft_name: String,
    pub art_nft_symbol: String,
    pub art_nft_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    MintArtist {
        artist: Addr,
    },
    MintArt {
        token_id: String,
        /// The owner of the newly minter NFT
        owner: String,
        /// Universal resource identifier for this NFT
        /// Should point to a JSON file that conforms to the ERC721
        /// Metadata JSON Schema
        token_uri: Option<String>,
    },
    ApproveCurator {
        dao: Addr,
        approved: u64,
        // Time in seconds
        duration: u64,
    },
    RevokeCurator {
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
    pub artist_nft_address: Option<Addr>,
    pub artist_nft_name: String,
    pub artist_nft_symbol: String,
    pub artist_nft_total_tokens_minted: u32,
    pub artist_nft_circulating_supply: u32,
}

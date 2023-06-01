use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub dao_members_code_id: u64,
    pub dao_multisig_code_id: u64,
    pub governance_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Identity {
    pub owner: Addr,
    pub name: String,
    pub id_type: IdType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IdType {
    User,
    Dao,
}

pub struct IdentityIndexes<'a> {
    // pk goes to second tuple element
    pub owner: UniqueIndex<'a, String, Identity, String>,
    pub name: UniqueIndex<'a, String, Identity, String>,
}

impl<'a> IndexList<Identity> for IdentityIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Identity>> + '_> {
        let v: Vec<&dyn Index<Identity>> = vec![&self.owner, &self.name];
        Box::new(v.into_iter())
    }
}

pub fn identities<'a>() -> IndexedMap<'a, String, Identity, IdentityIndexes<'a>> {
    let indexes = IdentityIndexes {
        owner: UniqueIndex::new(|d| d.owner.clone().to_string(), "identity"),
        name: UniqueIndex::new(|d| d.name.clone().to_string(), "identity"),
    };
    IndexedMap::new("identity", indexes)
}

pub const DAO_COUNT: Item<u64> = Item::new("dao_count");
pub const DAOS: Map<u64, Addr> = Map::new("daos");

pub fn next_dao_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = DAO_COUNT.may_load(store)?.unwrap_or_default() + 1;
    DAO_COUNT.save(store, &id)?;
    Ok(id)
}

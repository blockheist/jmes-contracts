use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub identityservice_contract: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const GRANT_COUNT: Item<u64> = Item::new("grant_count");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Grant {
    pub grant_id: u64,
    pub dao: Addr,
    pub amount_approved: Uint128,
    pub amount_remaining: Uint128,
    pub started: Timestamp,
    pub expires: Timestamp,
}

impl Grant {
    pub fn next_id(store: &mut dyn Storage) -> StdResult<u64> {
        let id: u64 = GRANT_COUNT.may_load(store)?.unwrap_or_default() + 1;
        GRANT_COUNT.save(store, &id)?;
        Ok(id)
    }
}

pub struct GrantIndexes<'a> {
    // pk goes to second tuple element
    pub dao: MultiIndex<'a, Addr, Grant, String>,
}

impl<'a> IndexList<Grant> for GrantIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Grant>> + '_> {
        let v: Vec<&dyn Index<Grant>> = vec![&self.dao];
        Box::new(v.into_iter())
    }
}

pub fn grants<'a>() -> IndexedMap<'a, String, Grant, GrantIndexes<'a>> {
    let indexes = GrantIndexes {
        dao: MultiIndex::new(|d: &Grant| d.dao.clone(), "grants", "grants__dao"),
    };
    IndexedMap::new("grants", indexes)
}


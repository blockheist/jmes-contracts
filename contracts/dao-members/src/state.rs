use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw4::{
    MEMBERS_CHANGELOG, MEMBERS_CHECKPOINTS, MEMBERS_KEY, TOTAL_KEY, TOTAL_KEY_CHANGELOG,
    TOTAL_KEY_CHECKPOINTS,
};
use cw_controllers::{Admin, Hooks};
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::{Duration, Threshold};

pub const ADMIN: Admin = Admin::new("admin");
pub const HOOKS: Hooks = Hooks::new("cw4-hooks");
pub const CONFIG: Item<Config> = Item::new("config");

pub const TOTAL: SnapshotItem<u64> = SnapshotItem::new(
    TOTAL_KEY,
    TOTAL_KEY_CHECKPOINTS,
    TOTAL_KEY_CHANGELOG,
    Strategy::EveryBlock,
);

pub const MEMBERS: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    MEMBERS_KEY,
    MEMBERS_CHECKPOINTS,
    MEMBERS_CHANGELOG,
    Strategy::EveryBlock,
);

#[cw_serde]
pub struct Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    pub dao_name: String,
}

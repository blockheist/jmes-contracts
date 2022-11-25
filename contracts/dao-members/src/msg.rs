use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use cw4::Member;
use cw_utils::{Duration, Threshold};

#[cw_serde]
pub struct DaoMembersInstantiateMsg {
    pub members: Vec<Member>,
    pub dao_name: String,
    pub threshold_percentage: Decimal,
    pub max_voting_period: Duration,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: Option<String> },
    /// apply a diff to the existing members.
    /// remove is applied after add, so if an address is in both, it is removed
    UpdateMembers {
        remove: Vec<String>,
        add: Vec<Member>,
    },
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub dao_name: String,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cw_controllers::AdminResponse)]
    Admin {},
    #[returns(cw4::TotalWeightResponse)]
    TotalWeight { at_height: Option<u64> },
    #[returns(cw4::MemberListResponse)]
    ListMembers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw4::MemberResponse)]
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks.
    #[returns(cw_controllers::HooksResponse)]
    Hooks {},
    #[returns(ConfigResponse)]
    Config {},
}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
    SubMsg, Uint64,
};
use cw2::set_contract_version;
use cw4::{
    Member, MemberChangedHookMsg, MemberDiff, MemberListResponse, MemberResponse,
    TotalWeightResponse,
};
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, Threshold};
use jmes::msg::GovernanceCoreSlotsResponse;

use crate::error::ContractError;
use crate::helpers::validate_unique_members;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, ADMIN, CONFIG, HOOKS, MEMBERS, TOTAL};
use jmes::constants::{MAX_DAO_MEMBERS, MIN_CORE_TEAM_MEMBERS};

// version info for migration info
const CONTRACT_NAME: &str = "dao-members";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    println!("\n\n info {:?}", info);

    // We're using AbsoluteCount as threshold so we can assign different voting power to each member
    // but we are limiting it to 100% max to have a more intuitive UX
    if msg.threshold_percentage > 100 {
        return Err(ContractError::InvalidThresholdPercentage {
            current: msg.threshold_percentage,
        });
    }

    CONFIG.save(
        deps.storage,
        &Config {
            threshold: Threshold::AbsoluteCount {
                weight: msg.threshold_percentage,
            },
            max_voting_period: msg.max_voting_period,
            dao_name: msg.dao_name,
            governance_addr: deps.api.addr_validate(msg.governance_addr.as_str())?,
        },
    )?;

    let admin = Some(info.sender.to_string()); // sender (identityservice) is temp admin so it can set dao-multisig as admin in the reply fn

    create(deps, admin, msg.members, env.block.height)?;
    Ok(Response::default())
}

// create is the instantiation logic with set_contract_version removed so it can more
// easily be imported in other contracts
pub fn create(
    mut deps: DepsMut,
    admin: Option<String>, // Eventually the admin is the dao-multisig address
    mut members: Vec<Member>,
    height: u64,
) -> Result<(), ContractError> {
    validate_unique_members(&mut members)?;

    if members.len() > MAX_DAO_MEMBERS {
        return Err(ContractError::TooManyMembers {
            max: MAX_DAO_MEMBERS,
            actual: members.len(),
        });
    }

    let members = members; // let go of mutability
    let admin_addr = admin
        .map(|admin| deps.api.addr_validate(&admin))
        .transpose()?;

    ADMIN.set(deps.branch(), admin_addr)?;

    let mut total = Uint64::zero();
    for member in members.into_iter() {
        let member_weight = Uint64::from(member.weight);
        total = total.checked_add(member_weight)?;
        let member_addr = deps.api.addr_validate(&member.addr)?;
        MEMBERS.save(deps.storage, &member_addr, &member_weight.u64(), height)?;
    }

    // We're using AbsoluteCount as threshold so we can assign different voting power to each member
    // but we are limiting it to 100% max to have a more intuitive UX
    if total.u64() > 100 {
        return Err(ContractError::InvalidThresholdPercentage {
            current: total.u64(),
        });
    }

    TOTAL.save(deps.storage, &total.u64(), height)?;

    Ok(())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => Ok(ADMIN.execute_update_admin(
            deps,
            info,
            admin.map(|admin| api.addr_validate(&admin)).transpose()?,
        )?),
        ExecuteMsg::UpdateMembers { add, remove } => {
            execute_update_members(deps, env, info, add, remove)
        }
        ExecuteMsg::AddHook { addr } => {
            Ok(HOOKS.execute_add_hook(&ADMIN, deps, info, api.addr_validate(&addr)?)?)
        }
        ExecuteMsg::RemoveHook { addr } => {
            Ok(HOOKS.execute_remove_hook(&ADMIN, deps, info, api.addr_validate(&addr)?)?)
        }
    }
}

pub fn execute_update_members(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    add: Vec<Member>,
    remove: Vec<String>,
) -> Result<Response, ContractError> {
    let attributes = vec![
        attr("action", "update_members"),
        attr("added", add.len().to_string()),
        attr("removed", remove.len().to_string()),
        attr("sender", &info.sender),
    ];

    // make the local update
    let diff = update_members(deps.branch(), env.block.height, info.sender, add, remove)?;
    // call all registered hooks
    let messages = HOOKS.prepare_hooks(deps.storage, |h| {
        diff.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;

    Ok(Response::new()
        .add_submessages(messages)
        .add_attributes(attributes))
}

// the logic from execute_update_members extracted for easier import
pub fn update_members(
    deps: DepsMut,
    height: u64,
    sender: Addr,
    mut to_add: Vec<Member>,
    to_remove: Vec<String>,
) -> Result<MemberChangedHookMsg, ContractError> {
    validate_unique_members(&mut to_add)?;

    let to_add = to_add; // let go of mutability

    ADMIN.assert_admin(deps.as_ref(), &sender)?;

    let mut total = Uint64::from(TOTAL.load(deps.storage)?);
    let mut diffs: Vec<MemberDiff> = vec![];

    // add all new members and update total
    for add in to_add.into_iter() {
        let add_addr = deps.api.addr_validate(&add.addr)?;
        MEMBERS.update(deps.storage, &add_addr, height, |old| -> StdResult<_> {
            total = total.checked_sub(Uint64::from(old.unwrap_or_default()))?;
            total = total.checked_add(Uint64::from(add.weight))?;
            diffs.push(MemberDiff::new(add.addr, old, Some(add.weight)));
            Ok(add.weight)
        })?;
    }

    for remove in to_remove.into_iter() {
        let remove_addr = deps.api.addr_validate(&remove)?;
        let old = MEMBERS.may_load(deps.storage, &remove_addr)?;
        // Only process this if they were actually in the list before
        if let Some(weight) = old {
            diffs.push(MemberDiff::new(remove, Some(weight), None));
            total = total.checked_sub(Uint64::from(weight))?;
            MEMBERS.remove(deps.storage, &remove_addr, height)?;
        }
    }

    let members: Vec<Member> = MEMBERS
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_DAO_MEMBERS + 1)
        .map(|item| {
            item.map(|(addr, weight)| Member {
                addr: addr.into(),
                weight,
            })
        })
        .collect::<StdResult<_>>()?;

    let governance_addr = CONFIG.load(deps.storage)?.governance_addr;

    let dao_multisig_addr = ADMIN.get(deps.as_ref())?.unwrap(); // At his point the admin is guaranteed to be set

    let core_slots: GovernanceCoreSlotsResponse = deps.querier.query_wasm_smart(
        governance_addr,
        &jmes::msg::GovernanceQueryMsg::CoreSlots {},
    )?;

    if Some(dao_multisig_addr.clone()) == core_slots.brand.as_ref().map(|s| s.dao.clone())
        || Some(dao_multisig_addr.clone()) == core_slots.core_tech.as_ref().map(|s| s.dao.clone())
        || Some(dao_multisig_addr.clone()) == core_slots.creative.as_ref().map(|s| s.dao.clone())
    {
        // Enforce Core Slot Membership rules
        // 1. A minimum of 3 members is required
        // 2. A maximum of 9 members is allowed
        // 3. The member with the largest weight must not reach the threshold

        if members.len() > MAX_DAO_MEMBERS || members.len() < MIN_CORE_TEAM_MEMBERS {
            return Err(ContractError::WrongCoreTeamMemberCount {
                min: MIN_CORE_TEAM_MEMBERS,
                max: MAX_DAO_MEMBERS,
            });
        }

        // find the member with the largest weight
        let max_weight = members.iter().map(|m| m.weight).max().unwrap_or_default();

        // TODO If in the future we use a different threshold for dao-members and dao-multisig,
        // we have to check both thresholds here:
        let config = CONFIG.load(deps.storage)?;

        // A single member weight is not allowed to reach the threshold
        // so if the threshold validates for a single member without an error -> we throw an error
        if config.threshold.validate(max_weight).is_ok() {
            return Err(ContractError::WrongCoreTeamMemberVotingPower {
                threshold: config.threshold,
                current: max_weight,
            });
        }
    } else {
        // We enforce Non-Core Slot Membership Rules
        if members.len() > MAX_DAO_MEMBERS {
            return Err(ContractError::TooManyMembers {
                max: MAX_DAO_MEMBERS,
                actual: members.len(),
            });
        }
    }

    TOTAL.save(deps.storage, &total.u64(), height)?;
    Ok(MemberChangedHookMsg { diffs })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Member {
            addr,
            at_height: height,
        } => to_binary(&query_member(deps, addr, height)?),
        QueryMsg::ListMembers { start_after, limit } => {
            to_binary(&query_list_members(deps, start_after, limit)?)
        }
        QueryMsg::TotalWeight { at_height: height } => {
            to_binary(&query_total_weight(deps, height)?)
        }
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
        QueryMsg::Hooks {} => to_binary(&HOOKS.query_hooks(deps)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_total_weight(deps: Deps, height: Option<u64>) -> StdResult<TotalWeightResponse> {
    let weight = match height {
        Some(h) => TOTAL.may_load_at_height(deps.storage, h),
        None => TOTAL.may_load(deps.storage),
    }?
    .unwrap_or_default();
    Ok(TotalWeightResponse { weight })
}

pub fn query_member(deps: Deps, addr: String, height: Option<u64>) -> StdResult<MemberResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let weight = match height {
        Some(h) => MEMBERS.may_load_at_height(deps.storage, &addr, h),
        None => MEMBERS.may_load(deps.storage, &addr),
    }?;
    Ok(MemberResponse { weight })
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        dao_name: config.dao_name,
        max_voting_period: config.max_voting_period,
        threshold: config.threshold,
    })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_list_members(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MemberListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let members = MEMBERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, weight)| Member {
                addr: addr.into(),
                weight,
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(MemberListResponse { members })
}

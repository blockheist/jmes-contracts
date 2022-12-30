// #[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Order};
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use cw_utils::parse_reply_instantiate_data;
use dao_multisig::msg::InstantiateResponse;
use dao_multisig::state::Executor;

use crate::error::ContractError;
use crate::msg::{
    DaosResponse, ExecuteMsg, GetIdentityByNameResponse, GetIdentityByOwnerResponse,
    InstantiateMsg, Ordering, QueryMsg,
};
use crate::state::{identities, next_dao_id, Config, IdType, Identity, CONFIG, DAOS};

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 20;

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_DAO_MEMBERS_REPLY_ID: u64 = 1u64;
const INSTANTIATE_DAO_MULTISIG_REPLY_ID: u64 = 2u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: msg.owner,
        dao_members_code_id: msg.dao_members_code_id,
        dao_multisig_code_id: msg.dao_multisig_code_id,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RegisterUser { name } => execute_register_user(deps, env, info, name),
        ExecuteMsg::RegisterDao(dao_members_instantiate_msg) => {
            execute_register_dao(deps, env, info, dao_members_instantiate_msg)
        }
    }
}

pub fn execute_register_user(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    // Validate requested identity name
    validate_name(&name)?;

    // Check if requesting address already has an identity registered
    let maybe_address_exists = identities().may_load(deps.storage, info.sender.to_string());

    if maybe_address_exists?.is_some() {
        return Err(ContractError::AlreadyRegistered {});
    }

    // Check if requested name is already taken
    let maybe_name_exists = identities().idx.name.item(deps.storage, name.clone());

    if maybe_name_exists?.is_some() {
        return Err(ContractError::NameTaken { name });
    }

    // Store requested name as an identity struct
    let identity = Identity {
        owner: info.sender,
        name,
        id_type: IdType::User,
    };

    identities().save(deps.storage, identity.owner.to_string(), &identity)?;

    Ok(Response::default())
}

pub fn execute_register_dao(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    dao_members_instantiate_msg: dao_members::msg::InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_name(&dao_members_instantiate_msg.dao_name)?;

    let config: Config = CONFIG.load(deps.storage)?;

    // Check if requested name is already taken
    let maybe_name_exists = identities()
        .idx
        .name
        .item(deps.storage, dao_members_instantiate_msg.dao_name.clone());

    if maybe_name_exists?.is_some() {
        return Err(ContractError::NameTaken {
            name: dao_members_instantiate_msg.dao_name,
        });
    }

    // Instantiate the DAO contract
    let instantiate_dao_members_message: WasmMsg = WasmMsg::Instantiate {
        label: "dao-members".to_string(),
        admin: None,
        code_id: config.dao_members_code_id,
        msg: to_binary(&dao_members_instantiate_msg)?,
        funds: vec![],
    };

    // Wrap DAO Instantiate Msg into a SubMsg
    let instantiate_dao_members_submsg: SubMsg = SubMsg {
        id: INSTANTIATE_DAO_MEMBERS_REPLY_ID,
        msg: instantiate_dao_members_message.into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    let response = Response::new().add_submessage(instantiate_dao_members_submsg);

    Ok(response)
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_DAO_MEMBERS_REPLY_ID => instantiate_dao_members_reply(deps, env, msg),
        INSTANTIATE_DAO_MULTISIG_REPLY_ID => instantiate_dao_multisig_reply(deps, env, msg),
        _ => Err(ContractError::Std(StdError::generic_err(
            "Unknown reply id.",
        ))),
    }
}

fn instantiate_dao_members_reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let reply = parse_reply_instantiate_data(msg).unwrap();
    let dao_members_addr = Addr::unchecked(&reply.contract_address);
    let dao_members_config: dao_members::msg::ConfigResponse = deps
        .querier
        .query_wasm_smart(&dao_members_addr, &dao_members::msg::QueryMsg::Config {})?;

    let dao_multisig_instantiate_msg = dao_multisig::msg::InstantiateMsg {
        group_addr: dao_members_addr.to_string(),
        executor: Some(Executor::Member),
        max_voting_period: dao_members_config.max_voting_period,
        threshold: dao_members_config.threshold,
        dao_name: dao_members_config.dao_name,
    };

    let instantiate_dao_multisig_message: WasmMsg = WasmMsg::Instantiate {
        label: "dao-multisig".to_string(),
        admin: None,
        code_id: config.dao_multisig_code_id,
        msg: to_binary(&dao_multisig_instantiate_msg)?,
        funds: vec![],
    };

    // Wrap dao-multisig InstantiateMsg into a SubMsg
    let instantiate_dao_multisig_submsg: SubMsg = SubMsg {
        id: INSTANTIATE_DAO_MULTISIG_REPLY_ID,
        msg: instantiate_dao_multisig_message.into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    Ok(Response::new().add_submessage(instantiate_dao_multisig_submsg))
}

fn instantiate_dao_multisig_reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let reply = parse_reply_instantiate_data(msg).unwrap();
    let dao_multisig_addr = Addr::unchecked(&reply.contract_address);
    let dao_multisig_config: dao_multisig::msg::ConfigResponse = deps
        .querier
        .query_wasm_smart(&dao_multisig_addr, &dao_multisig::msg::QueryMsg::Config {})?;

    let dao_name = dao_multisig_config.dao_name;

    // Update the admin of dao-members to the dao-multisig-addr
    let update_admin_msg = dao_members::msg::ExecuteMsg::UpdateAdmin {
        admin: Some(dao_multisig_addr.to_string()),
    };

    let update_admin_msg_wasm = WasmMsg::Execute {
        contract_addr: dao_multisig_config.dao_members_addr.into(),
        msg: to_binary(&update_admin_msg)?,
        funds: vec![],
    };

    // Store requested name as an identity struct
    let identity = Identity {
        owner: dao_multisig_addr.clone(),
        name: dao_name,
        id_type: IdType::Dao,
    };

    identities().save(deps.storage, identity.owner.to_string(), &identity)?;

    // This is used to allow paginating through all daos
    let dao_id = next_dao_id(deps.storage)?;
    DAOS.save(deps.storage, dao_id, &dao_multisig_addr)?;

    Ok(Response::new()
        .set_data(to_binary(&InstantiateResponse { dao_multisig_addr })?)
        .add_message(update_admin_msg_wasm))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetIdentityByOwner { owner } => to_binary(&query_identity_by_owner(deps, owner)?),
        QueryMsg::GetIdentityByName { name } => to_binary(&query_identity_by_name(deps, name)?),
        QueryMsg::Daos {
            start_after,
            limit,
            order,
        } => to_binary(&daos(deps, start_after, limit, order)?),
    }
}

pub fn daos(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    order: Option<Ordering>,
) -> StdResult<DaosResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let order = match order {
        Some(Ordering::Ascending) => Order::Ascending,
        Some(Ordering::Descending) => Order::Descending,
        _ => Order::Ascending,
    };

    let daos = DAOS
        .range(deps.storage, start, None, order)
        .take(limit)
        .collect::<StdResult<_>>()?;
    Ok(DaosResponse { daos })
}
fn query_identity_by_owner(deps: Deps, owner: String) -> StdResult<GetIdentityByOwnerResponse> {
    let maybe_identity = identities().may_load(deps.storage, owner)?;

    Ok(GetIdentityByOwnerResponse {
        identity: maybe_identity,
    })
}

fn query_identity_by_name(deps: Deps, name: String) -> StdResult<GetIdentityByNameResponse> {
    let maybe_identity_result = identities().idx.name.item(deps.storage, name);

    let maybe_identity = match maybe_identity_result? {
        Some(thing) => Some(thing.1),
        None => None,
    };

    Ok(GetIdentityByNameResponse {
        identity: maybe_identity,
    })
}

// let's not import a regexp library and just do these checks by hand
fn invalid_char(c: char) -> bool {
    let is_valid = c.is_digit(10) || c.is_ascii_lowercase() || (c == '.' || c == '-' || c == '_');
    !is_valid
}

/// validate_name returns an error if the name is invalid
/// (we require 3-20 lowercase ascii letters, numbers, or . - _)
fn validate_name(name: &str) -> Result<(), ContractError> {
    let length = name.len() as u64;
    if (name.len() as u64) < MIN_NAME_LENGTH {
        Err(ContractError::NameTooShort {
            length,
            min_length: MIN_NAME_LENGTH,
        })
    } else if (name.len() as u64) > MAX_NAME_LENGTH {
        Err(ContractError::NameTooLong {
            length,
            max_length: MAX_NAME_LENGTH,
        })
    } else {
        match name.find(invalid_char) {
            None => Ok(()),
            Some(bytepos_invalid_char_start) => {
                let c = name[bytepos_invalid_char_start..].chars().next().unwrap();
                Err(ContractError::InvalidCharacter { c })
            }
        }
    }
}

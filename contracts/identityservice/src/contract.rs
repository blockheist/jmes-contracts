// #[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use dao::msg::NameResponse;
use dao::msg::QueryMsg as DaoQueryMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, IdentityResponse, InstantiateMsg, QueryMsg};
use crate::state::{identities, Config, IdType, Identity, CONFIG};
use jmes::msg::DaoInstantiateMsg;

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 20;

// version info for migration info
const CONTRACT_NAME: &str = "identityservice";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_DAO_REPLY_ID: u64 = 1u64;

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
        dao_code_id: msg.dao_code_id,
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
        ExecuteMsg::RegisterDao(dao_instantiate_msg) => {
            execute_register_dao(deps, env, info, dao_instantiate_msg)
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
    info: MessageInfo,
    dao_instantiate_msg: DaoInstantiateMsg,
) -> Result<Response, ContractError> {
    // Validate requested identity name
    validate_name(&dao_instantiate_msg.dao_name)?;

    let config: Config = CONFIG.load(deps.storage)?;

    // Check if requesting address already has an identity registered
    let maybe_address_exists = identities().may_load(deps.storage, info.sender.to_string());

    if maybe_address_exists?.is_some() {
        return Err(ContractError::AlreadyRegistered {});
    }

    // Check if requested name is already taken
    let maybe_name_exists = identities()
        .idx
        .name
        .item(deps.storage, dao_instantiate_msg.dao_name.clone());

    if maybe_name_exists?.is_some() {
        return Err(ContractError::NameTaken {
            name: dao_instantiate_msg.dao_name,
        });
    }

    // Instantiate the DAO contract
    let instantiate_dao_message: WasmMsg = WasmMsg::Instantiate {
        label: "dao".to_string(),
        admin: None,
        code_id: config.dao_code_id,
        msg: to_binary(&dao_instantiate_msg)?,
        funds: vec![],
    };

    // Wrap DAO Instantiate Msg into a SubMsg
    let instantiate_dao_submsg: SubMsg = SubMsg {
        id: INSTANTIATE_DAO_REPLY_ID,
        msg: instantiate_dao_message.into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    let response = Response::new().add_submessage(instantiate_dao_submsg);

    Ok(response)
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_DAO_REPLY_ID => instantiate_dao_reply(deps, msg),
        _ => Err(ContractError::Std(StdError::generic_err(
            "Unknown reply id.",
        ))),
    }
}

fn instantiate_dao_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let reply = parse_reply_instantiate_data(msg).unwrap();
    let dao_addr = Addr::unchecked(&reply.contract_address);
    // Read name from the dao contract config
    let name_response: NameResponse = deps
        .querier
        .query_wasm_smart(&dao_addr, &DaoQueryMsg::Name {})
        .unwrap();

    let name = name_response.name.unwrap();

    // Store requested name as an identity struct
    let identity = Identity {
        owner: dao_addr,
        name,
        id_type: IdType::Dao,
    };

    identities().save(deps.storage, identity.owner.to_string(), &identity)?;

    Ok(Response::new().add_attribute("reply", format!("{:?}", reply)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetIdentityByOwner { owner } => to_binary(&query_identity_by_owner(deps, owner)?),
        QueryMsg::GetIdentityByName { name } => to_binary(&query_identity_by_name(deps, name)?),
    }
}

fn query_identity_by_owner(deps: Deps, owner: String) -> StdResult<IdentityResponse> {
    let maybe_identity = identities().may_load(deps.storage, owner)?;

    Ok(IdentityResponse {
        identity: maybe_identity,
    })
}

fn query_identity_by_name(deps: Deps, name: String) -> StdResult<IdentityResponse> {
    let maybe_identity_result = identities().idx.name.item(deps.storage, name);

    let maybe_identity = match maybe_identity_result? {
        Some(thing) => Some(thing.1),
        None => None,
    };

    Ok(IdentityResponse {
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

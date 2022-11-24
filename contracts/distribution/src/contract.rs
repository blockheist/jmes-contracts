use std::env;
use std::ops::{Mul, Sub};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, GrantResponse, GrantsResponse, InstantiateMsg, QueryMsg,
};
use crate::state::{grants, Config, Grant, CONFIG};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Timestamp, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 100;

const MIN_CLAIMABLE_AMOUNT: Uint128 = Uint128::new(50000 as u128);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: msg.owner,
        identityservice_contract: msg.identityservice_contract,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Grant { grant_id } => to_binary(&query_grant(deps, env, grant_id)?),
        QueryMsg::Grants {
            dao,
            start_after,
            limit,
        } => to_binary(&query_grants(deps, dao, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    println!("config {:?}", config);
    Ok(ConfigResponse {
        owner: config.owner,
        identityservice_contract: config.identityservice_contract,
    })
}

fn query_grant(deps: Deps, env: Env, grant_id: u64) -> StdResult<Option<GrantResponse>> {
    let maybe_grant = grants().may_load(deps.storage, grant_id.to_string())?;
    if maybe_grant.is_none() {
        return Ok(None);
    }

    let grant = maybe_grant.unwrap();

    let claimable_amount = claimable_amount(env.block.time, &grant);

    let grant_response = GrantResponse {
        grant_id: grant.grant_id,
        dao: grant.dao,
        amount_approved: grant.amount_approved,
        amount_remaining: grant.amount_remaining,
        started: grant.started,
        expires: grant.expires,
        claimable_amount,
    };

    Ok(Some(grant_response))
}

fn query_grants(
    deps: Deps,
    dao: Option<Addr>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<GrantsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    // Select index range for dao query parameter
    let range = match dao {
        None => grants().range(deps.storage, start, None, Order::Ascending),
        Some(dao_addr) => {
            grants()
                .idx
                .dao
                .prefix(dao_addr)
                .range(deps.storage, start, None, Order::Ascending)
        }
    };

    let grants: StdResult<Vec<Grant>> = range
        .take(limit)
        .map(|item| item.map(|(_, grant)| grant))
        .collect();

    let res = GrantsResponse { grants: grants? };

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddGrant {
            dao,
            duration,
            amount,
        } => execute_add_grant(deps, env, info, dao, duration, amount),
        // ExecuteMsg::RevokeGrant { dao } => execute_revoke_grant(deps, env, info, dao),
        ExecuteMsg::Claim { grant_id } => execute_claim(deps, env, info, grant_id),
    }
}

pub fn execute_add_grant(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    dao: Addr,
    duration: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only the governance contract can add grants via a funding proposal
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let grant = Grant {
        grant_id: Grant::next_id(deps.storage)?,
        dao: dao.clone(),
        amount_approved: amount,
        amount_remaining: amount,
        started: env.block.time,
        expires: env.block.time.plus_seconds(duration),
    };

    grants().save(deps.storage, grant.grant_id.to_string(), &grant)?;

    Ok(Response::new()
        .add_attribute("grant_id", grant.grant_id.to_string())
        .add_attribute("dao", grant.dao.to_string())
        .add_attribute("amount_approved", grant.amount_approved.to_string())
        .add_attribute("amount_remaining", grant.amount_remaining.to_string())
        .add_attribute("started", grant.started.to_string())
        .add_attribute("expires", grant.expires.to_string())
        .add_attribute("owner", config.owner)
        .add_attribute("sender", info.sender))
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    grant_id: u64,
) -> Result<Response, ContractError> {
    // Anyone can claim, they are doing us a favor by paying the tx fee

    let maybe_grant = grants().may_load(deps.storage, grant_id.to_string())?;

    if maybe_grant.is_none() {
        return Err(ContractError::GrantNotFound {});
    }

    let mut grant = maybe_grant.unwrap();

    if grant.amount_remaining.is_zero() {
        return Err(ContractError::AlreadyClaimed {});
    }

    let claimable_amount = claimable_amount(env.block.time, &grant);

    grant.amount_remaining = grant.amount_remaining.sub(claimable_amount);

    grants().save(deps.storage, grant.grant_id.to_string(), &grant)?;

    // TODO returning a contract error throws undescript error 400
    if claimable_amount < MIN_CLAIMABLE_AMOUNT {
        return Err(ContractError::AmountTooSmall {});
    }

    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: grant.dao.to_string(),
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: claimable_amount,
            }],
        })
        .add_attribute("claimable_amount", claimable_amount)
        .add_attribute("amount_remaining", grant.amount_remaining))
}

fn claimable_amount(block_time: Timestamp, grant: &Grant) -> Uint128 {
    let time_passed = block_time.minus_nanos(grant.started.nanos()).seconds(); // time since start of grant
    let duration = grant.expires.minus_nanos(grant.started.nanos()).seconds(); // lifespan of grant

    let matured_permille = (time_passed * 1000 / duration).min(1000); // Max pay out 1000/permille
    let matured_amount = grant // Total amount matured: 'amount * time since start'/'lifespan'
        .amount_approved
        .mul(Decimal::permille(matured_permille));

    let already_claimed = grant.amount_approved.sub(grant.amount_remaining);
    let claimable_amount = matured_amount.sub(already_claimed);

    return claimable_amount;
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
//     use cosmwasm_std::{from_binary, to_binary, SubMsgResponse, SubMsgResult};
//     use prost::Message;

//     const NFT_CONTRACT_ADDR: &str = "nftcontract";

//     // Type for replies to contract instantiate msgs
//     #[derive(Clone, PartialEq, Message)]
//     struct MsgInstantiateContractResponse {
//         #[prost(string, tag = "1")]
//         pub contract_address: ::prost::alloc::string::String,
//         #[prost(bytes, tag = "2")]
//         pub data: ::prost::alloc::vec::Vec<u8>,
//     }

//     #[test]
//     fn initialization() {
//         let mut deps = mock_dependencies();
//         let msg = InstantiateMsg {
//             owner: Addr::unchecked("owner"),
//             artist_nft_name: String::from("Artist Nft"),
//             artist_nft_symbol: String::from("artistnft"),
//             artist_nft_code_id: 10u64,
//             artist_nft_token_uri: String::from("http://artist-nft.art/"),
//             artist_nft_extension: None,
//         };

//         let info = mock_info("owner", &[]);
//         let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

//         assert_eq!(
//             res.messages,
//             vec![SubMsg {
//                 msg: WasmMsg::Instantiate {
//                     code_id: msg.artist_nft_code_id,
//                     msg: to_binary(&Cw721InstantiateMsg {
//                         name: msg.name.clone(),
//                         symbol: msg.symbol.clone(),
//                         minter: MOCK_CONTRACT_ADDR.to_string(),
//                     })
//                     .unwrap(),
//                     funds: vec![],
//                     admin: None,
//                     label: String::from("Artist NFT"),
//                 }
//                 .into(),
//                 id: INSTANTIATE_TOKEN_REPLY_ID,
//                 gas_limit: None,
//                 reply_on: ReplyOn::Success,
//             }]
//         );

//         let instantiate_reply = MsgInstantiateContractResponse {
//             contract_address: "nftcontract".to_string(),
//             data: vec![2u8; 32769],
//         };
//         let mut encoded_instantiate_reply =
//             Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
//         instantiate_reply
//             .encode(&mut encoded_instantiate_reply)
//             .unwrap();

//         let reply_msg = Reply {
//             id: INSTANTIATE_TOKEN_REPLY_ID,
//             result: SubMsgResult::Ok(SubMsgResponse {
//                 events: vec![],
//                 data: Some(encoded_instantiate_reply.into()),
//             }),
//         };
//         reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

//         let query_msg = QueryMsg::Config {};
//         let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
//         let config: Config = from_binary(&res).unwrap();
//         assert_eq!(
//             config,
//             Config {
//                 owner: Addr::unchecked("owner"),
//                 artist_nft_address: Some(Addr::unchecked(NFT_CONTRACT_ADDR)),
//                 name: msg.name,
//                 symbol: msg.symbol,
//                 token_uri: msg.token_uri,
//                 extension: None,
//                 total_tokens_minted: 0,
//                 circulating_supply: 0
//             }
//         );
//     }

//     // #[test]
//     // fn mint() {
//     //     let mut deps = mock_dependencies();
//     //     let msg = InstantiateMsg {
//     //         owner: Addr::unchecked("owner"),
//     //         name: String::from("Artist Nft"),
//     //         symbol: String::from("artistnft"),
//     //         artist_nft_code_id: 10u64,
//     //         token_uri: String::from("http://artist-nft.art/"),
//     //         extension: None,
//     //     };

//     //     let info = mock_info("owner", &[]);
//     //     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//     //     let instantiate_reply = MsgInstantiateContractResponse {
//     //         contract_address: NFT_CONTRACT_ADDR.to_string(),
//     //         data: vec![2u8; 32769],
//     //     };
//     //     let mut encoded_instantiate_reply =
//     //         Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
//     //     instantiate_reply
//     //         .encode(&mut encoded_instantiate_reply)
//     //         .unwrap();

//     //     let reply_msg = Reply {
//     //         id: INSTANTIATE_TOKEN_REPLY_ID,
//     //         result: SubMsgResult::Ok(SubMsgResponse {
//     //             events: vec![],
//     //             data: Some(encoded_instantiate_reply.into()),
//     //         }),
//     //     };
//     //     reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

//     //     let msg = ExecuteMsg::MintArtist {
//     //         artist: Addr::unchecked("artistaddress"),
//     //     };

//     //     let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
//     //     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     //     let mint_msg = ();
//     //     assert_eq!(
//     //         res.messages[0],
//     //         SubMsg {
//     //             msg: CosmosMsg::Wasm(WasmMsg::Execute {
//     //                 contract_addr: NFT_CONTRACT_ADDR.to_string(),
//     //                 msg: to_binary(&mint_msg).unwrap(),
//     //                 funds: vec![],
//     //             }),
//     //             id: 0,
//     //             gas_limit: None,
//     //             reply_on: ReplyOn::Never,
//     //         }
//     //     );
//     // }
//     #[test]
//     fn invalid_reply_id() {
//         let mut deps = mock_dependencies();
//         let msg = InstantiateMsg {
//             owner: Addr::unchecked("owner"),
//             name: String::from("Artist Nft"),
//             symbol: String::from("artistnft"),
//             artist_nft_code_id: 10u64,
//             token_uri: String::from("http://artist-nft.art/"),
//             extension: None,
//         };

//         let info = mock_info("owner", &[]);
//         instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let instantiate_reply = MsgInstantiateContractResponse {
//             contract_address: NFT_CONTRACT_ADDR.to_string(),
//             data: vec![2u8; 32769],
//         };
//         let mut encoded_instantiate_reply =
//             Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
//         instantiate_reply
//             .encode(&mut encoded_instantiate_reply)
//             .unwrap();

//         let reply_msg = Reply {
//             id: 10,
//             result: SubMsgResult::Ok(SubMsgResponse {
//                 events: vec![],
//                 data: Some(encoded_instantiate_reply.into()),
//             }),
//         };
//         let err = reply(deps.as_mut(), mock_env(), reply_msg).unwrap_err();
//         match err {
//             ContractError::InvalidTokenReplyId {} => {}
//             e => panic!("unexpected error: {}", e),
//         }
//     }

//     // #[test]
//     // fn cw721_already_linked() {
//     //     let mut deps = mock_dependencies();
//     //     let msg = InstantiateMsg {
//     //         owner: Addr::unchecked("owner"),
//     //         max_tokens: 1,
//     //         unit_price: Uint128::new(1),
//     //         name: String::from("SYNTH"),
//     //         symbol: String::from("SYNTH"),
//     //         artist_nft_code_id: 10u64,
//     //         cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//     //         token_uri: String::from("https://ipfs.io/ipfs/Q"),
//     //         extension: None,
//     //     };

//     //     let info = mock_info("owner", &[]);
//     //     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//     //     let instantiate_reply = MsgInstantiateContractResponse {
//     //         contract_address: NFT_CONTRACT_ADDR.to_string(),
//     //         data: vec![2u8; 32769],
//     //     };
//     //     let mut encoded_instantiate_reply =
//     //         Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
//     //     instantiate_reply
//     //         .encode(&mut encoded_instantiate_reply)
//     //         .unwrap();

//     //     let reply_msg = Reply {
//     //         id: 1,
//     //         result: SubMsgResult::Ok(SubMsgResponse {
//     //             events: vec![],
//     //             data: Some(encoded_instantiate_reply.into()),
//     //         }),
//     //     };
//     //     reply(deps.as_mut(), mock_env(), reply_msg.clone()).unwrap();

//     //     let err = reply(deps.as_mut(), mock_env(), reply_msg).unwrap_err();
//     //     match err {
//     //         ContractError::Cw721AlreadyLinked {} => {}
//     //         e => panic!("unexpected error: {}", e),
//     //     }
//     // }

//     // #[test]
//     // fn sold_out() {
//     //     let mut deps = mock_dependencies();
//     //     let msg = InstantiateMsg {
//     //         owner: Addr::unchecked("owner"),
//     //         max_tokens: 1,
//     //         unit_price: Uint128::new(1),
//     //         name: String::from("SYNTH"),
//     //         symbol: String::from("SYNTH"),
//     //         artist_nft_code_id: 10u64,
//     //         cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//     //         token_uri: String::from("https://ipfs.io/ipfs/Q"),
//     //         extension: None,
//     //     };

//     //     let info = mock_info("owner", &[]);
//     //     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//     //     let instantiate_reply = MsgInstantiateContractResponse {
//     //         contract_address: NFT_CONTRACT_ADDR.to_string(),
//     //         data: vec![2u8; 32769],
//     //     };
//     //     let mut encoded_instantiate_reply =
//     //         Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
//     //     instantiate_reply
//     //         .encode(&mut encoded_instantiate_reply)
//     //         .unwrap();

//     //     let reply_msg = Reply {
//     //         id: INSTANTIATE_TOKEN_REPLY_ID,
//     //         result: SubMsgResult::Ok(SubMsgResponse {
//     //             events: vec![],
//     //             data: Some(encoded_instantiate_reply.into()),
//     //         }),
//     //     };
//     //     reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

//     //     let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
//     //         sender: String::from("minter"),
//     //         amount: Uint128::new(1),
//     //         msg: [].into(),
//     //     });
//     //     let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

//     //     // Max mint is 1, so second mint request should fail
//     //     execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
//     //     let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

//     //     match err {
//     //         ContractError::SoldOut {} => {}
//     //         e => panic!("unexpected error: {}", e),
//     //     }
//     // }

//     // #[test]
//     // fn uninitialized() {
//     //     // Config has not been fully initialized with nft contract address via instantiation reply
//     //     let mut deps = mock_dependencies();
//     //     let msg = InstantiateMsg {
//     //         owner: Addr::unchecked("owner"),
//     //         max_tokens: 1,
//     //         unit_price: Uint128::new(1),
//     //         name: String::from("SYNTH"),
//     //         symbol: String::from("SYNTH"),
//     //         artist_nft_code_id: 10u64,
//     //         cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//     //         token_uri: String::from("https://ipfs.io/ipfs/Q"),
//     //         extension: None,
//     //     };

//     //     let info = mock_info("owner", &[]);
//     //     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     //     // Test token transfer when nft contract has not been linked

//     //     let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
//     //         sender: String::from("minter"),
//     //         amount: Uint128::new(1),
//     //         msg: [].into(),
//     //     });
//     //     let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

//     //     let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
//     //     match err {
//     //         ContractError::Uninitialized {} => {}
//     //         e => panic!("unexpected error: {}", e),
//     //     }
//     // }

//     // #[test]
//     // fn unauthorized_token() {
//     //     let mut deps = mock_dependencies();
//     //     let msg = InstantiateMsg {
//     //         owner: Addr::unchecked("owner"),
//     //         name: String::from("Artist Nft"),
//     //         symbol: String::from("artistnft"),
//     //         artist_nft_code_id: 10u64,
//     //         token_uri: String::from("http://artist-nft.art/"),
//     //         extension: None,
//     //     };

//     //     let info = mock_info("owner", &[]);
//     //     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     //     // Link nft token contract using reply

//     //     let instantiate_reply = MsgInstantiateContractResponse {
//     //         contract_address: NFT_CONTRACT_ADDR.to_string(),
//     //         data: vec![2u8; 32769],
//     //     };
//     //     let mut encoded_instantiate_reply =
//     //         Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
//     //     instantiate_reply
//     //         .encode(&mut encoded_instantiate_reply)
//     //         .unwrap();

//     //     let reply_msg = Reply {
//     //         id: INSTANTIATE_TOKEN_REPLY_ID,
//     //         result: SubMsgResult::Ok(SubMsgResponse {
//     //             events: vec![],
//     //             data: Some(encoded_instantiate_reply.into()),
//     //         }),
//     //     };
//     //     let reply = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
//     //     print!("reply {:?}", reply);

//     //     // Test token transfer from invalid token contract
//     //     let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
//     //         sender: String::from("minter"),
//     //         amount: Uint128::new(1),
//     //         msg: [].into(),
//     //     });
//     //     let info = mock_info("unauthorized-token", &[]);
//     //     let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

//     //     match err {
//     //         ContractError::UnauthorizedTokenContract {} => {}
//     //         e => panic!("unexpected error: {}", e),
//     //     }
//     // }

//     // #[test]
//     // fn wrong_amount() {
//     //     let mut deps = mock_dependencies();
//     //     let msg = InstantiateMsg {
//     //         owner: Addr::unchecked("owner"),
//     //         max_tokens: 1,
//     //         unit_price: Uint128::new(1),
//     //         name: String::from("SYNTH"),
//     //         symbol: String::from("SYNTH"),
//     //         artist_nft_code_id: 10u64,
//     //         cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//     //         token_uri: String::from("https://ipfs.io/ipfs/Q"),
//     //         extension: None,
//     //     };

//     //     let info = mock_info("owner", &[]);
//     //     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     //     // Link nft token contract using reply

//     //     let instantiate_reply = MsgInstantiateContractResponse {
//     //         contract_address: NFT_CONTRACT_ADDR.to_string(),
//     //         data: vec![2u8; 32769],
//     //     };
//     //     let mut encoded_instantiate_reply =
//     //         Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
//     //     instantiate_reply
//     //         .encode(&mut encoded_instantiate_reply)
//     //         .unwrap();

//     //     let reply_msg = Reply {
//     //         id: INSTANTIATE_TOKEN_REPLY_ID,
//     //         result: SubMsgResult::Ok(SubMsgResponse {
//     //             events: vec![],
//     //             data: Some(encoded_instantiate_reply.into()),
//     //         }),
//     //     };
//     //     reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

//     //     // Test token transfer from invalid token contract
//     //     let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
//     //         sender: String::from("minter"),
//     //         amount: Uint128::new(100),
//     //         msg: [].into(),
//     //     });
//     //     let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
//     //     let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

//     //     match err {
//     //         ContractError::WrongPaymentAmount {} => {}
//     //         e => panic!("unexpected error: {}", e),
//     //     }
//     // }
// }

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Approval, Config, APPROVALS, CONFIG};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdResult,
    SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw721_metadata_onchain::{
    ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg, Metadata,
};
use identityservice::msg::QueryMsg::GetIdentityByOwner;
use identityservice::state::IdType::Dao;

use cw_utils::parse_reply_instantiate_data;
use identityservice::msg::GetIdentityByOwnerResponse;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_ART_NFT_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        art_nft_address: None,
        owner: msg.owner,
        identityservice_contract: msg.identityservice_contract,
        art_nft_name: msg.art_nft_name.clone(),
        art_nft_symbol: msg.art_nft_symbol.clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.art_nft_code_id,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.art_nft_name.clone(),
                symbol: msg.art_nft_symbol,
                minter: env.contract.address.to_string(),
            })?,
            funds: vec![],
            admin: None,
            label: String::from("Art NFT"),
        }
        .into(),
        id: INSTANTIATE_ART_NFT_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_ART_NFT_REPLY_ID => instantiate_art_nft_reply(deps, msg),
        _ => Err(ContractError::InvalidTokenReplyId {}),
    }
}

fn instantiate_art_nft_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.art_nft_address != None {
        return Err(ContractError::Cw721AlreadyLinked {});
    }

    let reply = parse_reply_instantiate_data(msg).unwrap();
    config.art_nft_address = Addr::unchecked(reply.contract_address).into();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    println!("config {:?}", config);
    Ok(ConfigResponse {
        owner: config.owner,
        identityservice_contract: config.identityservice_contract,
        art_nft_address: config.art_nft_address,
        art_nft_name: config.art_nft_name,
        art_nft_symbol: config.art_nft_symbol,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MintArt {
            token_id,
            owner,
            token_uri,
            metadata,
        } => execute_mint_art(deps, env, info, token_id, owner, token_uri, metadata),
        ExecuteMsg::ApproveDealer {
            dao,
            approved, // amount of NFTs to be minted
            duration, // in blocks
        } => execute_approve_dealer(deps, env, info, dao, approved, duration),
        ExecuteMsg::RevokeDealer { dao } => execute_revoke_dealer(deps, env, info, dao),
    }
}

pub fn execute_approve_dealer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    dao: Addr,
    approved: u64, // amount of NFTs to be minted
    duration: u64, // in blocks
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let expires = env.block.height + duration;
    let approval = Approval {
        approved,
        minted: 0,
        burned: 0,
        expires,
    };
    APPROVALS.save(deps.storage, &dao, &approval)?;
    Ok(Response::new()
        .add_attribute("block_time", env.block.time.to_string())
        .add_attribute("expires", expires.to_string())
        .add_attribute("owner", config.owner)
        .add_attribute("sender", info.sender))
}

pub fn execute_revoke_dealer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    dao: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::UnauthorizedTokenContract {}); // TODO: change error
    }

    APPROVALS.remove(deps.storage, &dao);
    Ok(Response::new())
}

pub fn execute_mint_art(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    owner: String,
    token_uri: Option<String>,
    metadata: Option<Metadata>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let art_nft_address = config.art_nft_address.clone().unwrap();

    assert_nft_contracts_are_linked(&config).unwrap();

    // Ensure info.sender is a DAO
    let maybe_identity_resp: GetIdentityByOwnerResponse = deps.querier.query_wasm_smart(
        config.identityservice_contract.clone(),
        &GetIdentityByOwner {
            owner: info.sender.clone().into(),
        },
    )?;

    let maybe_identity = maybe_identity_resp.identity;

    match maybe_identity {
        Some(identity) => {
            if identity.id_type != Dao {
                return Err(ContractError::Unauthorized {});
            }
        }
        None => return Err(ContractError::Unauthorized {}),
    }

    let maybe_approval = APPROVALS.may_load(deps.storage, &info.sender)?;

    // Assert sender has art dealer approval
    if maybe_approval.is_none() {
        return Err(ContractError::DealerNotApproved {});
    }

    let mut approval = maybe_approval.unwrap();

    // Assert approval is not expired
    if env.block.height > approval.expires {
        return Err(ContractError::ApprovalExpired {});
    }

    // Assert approved amount is not exceeded
    if approval.minted + 1 > approval.approved {
        return Err(ContractError::ApprovedExceeded {});
    }

    approval.minted += 1;
    APPROVALS.save(deps.storage, &info.sender, &approval)?;

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id,
        owner,
        token_uri,
        extension: metadata,
    };

    let callback = WasmMsg::Execute {
        contract_addr: art_nft_address.to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    };

    Ok(Response::new().add_message(callback))
}

fn assert_nft_contracts_are_linked(config: &Config) -> Result<(), ContractError> {
    if config.art_nft_address.is_none() {
        return Err(ContractError::Cw721NotLinked {});
    }
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
//     use cosmwasm_std::{from_binary, to_binary, CosmosMsg, SubMsgResponse, SubMsgResult};
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

//         let query_msg = QueryMsg::GetConfig {};
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

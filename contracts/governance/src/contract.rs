use crate::error::ContractError;
// use crate::msg::Feature::ArtistCurator;
use crate::msg::{ExecuteMsg, InstantiateMsg, ProposalMsg, QueryMsg};
use crate::state::{
    Config, CoreSlots, WinningGrant, CONFIG, CORE_SLOTS, PROPOSAL_COUNT, WINNING_GRANTS,
};
use artist_curator::msg::ExecuteMsg::ApproveCurator;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use identityservice::msg::QueryMsg::GetIdentityByOwner;
use identityservice::state::IdType::Dao;

// Address for burning the proposal fee
const BURN_ADDRESS: &str = "jmes1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqf5laz2";

const MIN_VOTE_COINS: u128 = 1000_000_000u128; // 1.000 bJMES

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Default pagination constants
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let owner_addr = deps.api.addr_validate(&msg.owner)?;

    let config = Config {
        owner: Some(owner_addr),
        artist_curator_addr: None,
        identityservice_addr: None,
        proposal_required_deposit: msg.proposal_required_deposit,
        proposal_required_percentage: msg.proposal_required_percentage, // 10
        period_start_epoch: msg.period_start_epoch,                     // 1660000000,
        posting_period_length: msg.posting_period_length,               // 300000,
        voting_period_length: msg.voting_period_length,                 // 606864,
    };

    CONFIG.save(deps.storage, &config)?;

    CORE_SLOTS.save(
        deps.storage,
        &CoreSlots {
            brand: None,
            core_tech: None,
            creative: None,
        },
    )?;

    // TODO Remove mock winning grants
    // START MOCK WINNING GRANTS
    let mut mock_winning_grants: Vec<WinningGrant> = vec![];

    mock_winning_grants.push(WinningGrant {
        dao: Addr::unchecked("jmes1lzs0l3h9q7003ugspe8x8ueug9j6n4hau5pyha"),
        amount: Uint128::from(100u128),
        expire_at_height: 260010,
        yes_ratio: Decimal::percent(85),
    });

    mock_winning_grants.push(WinningGrant {
        dao: Addr::unchecked("jmes1cs0sav8qwsdzqt8ep2wfp5h830c6heq84pxmjq"),
        amount: Uint128::from(200u128),
        expire_at_height: 260020,
        yes_ratio: Decimal::percent(90),
    });

    mock_winning_grants.push(WinningGrant {
        dao: Addr::unchecked("jmes1wcf03kqs6klcggkf55nynueggjn8hxw47gtzra"),
        amount: Uint128::from(300u128),
        expire_at_height: 260000,
        yes_ratio: Decimal::percent(95),
    });

    WINNING_GRANTS.save(deps.storage, &mock_winning_grants)?;

    // END MOCK WINNING GRANTS

    // WINNING_GRANTS.save(deps.storage, &vec![])?; // TODO enable for production when MOCK WINNING GRANTS are removed

    PROPOSAL_COUNT.save(deps.storage, &(0 as u64))?;
    Ok(Response::new())
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Config {} => to_binary(&CONFIG.load(deps.storage)?),
        PeriodInfo {} => to_binary(&query::period_info(deps, env)?),
        Proposal { id } => to_binary(&query::proposal(deps, env, id)?),
        Proposals { start, limit } => to_binary(&query::proposals(deps, env, start, limit)?),
        CoreSlots {} => to_binary(&query::core_slots(deps, env)?),
        WinningGrants {} => to_binary(&query::winning_grants(deps, env)?),
    }
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        Propose(proposal_msg) => exec::proposal(deps, env, info, proposal_msg),
        Vote { id, vote } => exec::vote(deps, env, info, id, vote),
        Conclude { id } => exec::conclude(deps, env, id),
        SetCoreSlot { proposal_id } => exec::set_core_slot(deps, env, info, proposal_id),
        UnsetCoreSlot { proposal_id } => exec::unset_core_slot(deps, env, info, proposal_id),
        ResignCoreSlot { slot, note } => exec::resign_core_slot(deps, env, info, slot, note),
        SetContract {
            artist_curator,
            identityservice,
        } => exec::set_contract(deps, env, info, artist_curator, identityservice),
    }
}

mod exec {
    use cosmwasm_std::{BankMsg, Coin, CosmosMsg, Decimal, Uint128, WasmMsg};
    use identityservice::msg::GetIdentityByOwnerResponse;

    use super::*;

    use crate::contract::query::period_info;
    use crate::msg::{CoreSlot, Feature, PeriodInfoResponse, ProposalPeriod, RevokeCoreSlot};
    use crate::state::{
        Funding, ProposalStatus, SlotVoteResult, WinningGrant, CORE_SLOTS, WINNING_GRANTS,
    };
    use crate::state::{
        Proposal, ProposalType,
        VoteOption::{self, *},
        PROPOSALS,
    };

    pub fn proposal(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        proposal_msg: ProposalMsg,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;

        // Check if governance is already enabled (ensures fair distribution of coins for proposal deposit)
        if env.block.time.seconds() <= config.period_start_epoch {
            return Err(ContractError::TooEarly {
                start_epoch: config.period_start_epoch,
            });
        }

        // Only DAO identities are allowed to post proposals
        let maybe_identity_resp: GetIdentityByOwnerResponse = deps.querier.query_wasm_smart(
            config.clone().identityservice_addr.unwrap().clone(),
            &GetIdentityByOwner {
                owner: info.sender.clone().into(),
            },
        )?;

        let maybe_identity = maybe_identity_resp.identity;

        if maybe_identity.is_none() || maybe_identity.unwrap().id_type != Dao {
            return Err(ContractError::Unauthorized {});
        }

        // Only during a posting period can new proposals be posted
        let period_info = period_info(deps.as_ref(), env.clone())?;

        if period_info.current_period != ProposalPeriod::Posting {
            return Err(ContractError::NotPostingPeriod {});
        }

        // A proposal fee must be paid when posting a proposal
        // TODO The fee must be burned (should be attached msg to burning address)
        let deposit_amount = info
            .funds
            .iter()
            .find(|coin| coin.denom == "ujmes")
            .unwrap()
            .amount;
        if deposit_amount < Uint128::from(config.proposal_required_deposit) {
            return Err(ContractError::InsufficientDeposit {});
        }

        match proposal_msg {
            ProposalMsg::TextProposal {
                title,
                description,
                funding,
            } => text_proposal(
                deps,
                info,
                env,
                config,
                period_info,
                deposit_amount,
                title,
                description,
                funding,
            ),
            ProposalMsg::RequestFeature {
                title,
                description,
                funding,
                feature,
            } => request_feature(
                deps,
                info,
                env,
                config,
                period_info,
                deposit_amount,
                title,
                description,
                funding,
                feature,
            ),

            ProposalMsg::Improvement {
                title,
                description,
                funding,
                msgs,
            } => improvement(
                deps,
                info,
                env,
                config,
                period_info,
                deposit_amount,
                title,
                description,
                funding,
                msgs,
            ),
            ProposalMsg::CoreSlot {
                title,
                description,
                funding,
                slot,
            } => core_slot(
                deps,
                info,
                env,
                config,
                period_info,
                deposit_amount,
                title,
                description,
                funding,
                slot,
            ),
            ProposalMsg::RevokeCoreSlot {
                title,
                description,
                funding,
                revoke_slot,
            } => revoke_core_slot(
                deps,
                info,
                env,
                config,
                period_info,
                deposit_amount,
                title,
                description,
                funding,
                revoke_slot,
            ),
        }
    }

    pub fn text_proposal(
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        _config: Config,
        period_info: PeriodInfoResponse,
        deposit_amount: Uint128,
        title: String,
        description: String,
        funding: Option<Funding>,
    ) -> Result<Response, ContractError> {
        let id = Proposal::next_id(deps.storage)?;
        let proposal = Proposal {
            id,
            dao: info.sender,
            title,
            description,
            prop_type: ProposalType::Text {},
            coins_no: Uint128::zero(),
            coins_yes: Uint128::zero(),
            yes_voters: Vec::new(),
            no_voters: Vec::new(),
            deposit_amount,
            start_block: env.block.height, // used for voting coin lookup
            posting_start: period_info.current_posting_start,
            voting_start: period_info.current_voting_start,
            voting_end: period_info.current_voting_end,
            concluded: false,
            funding,
            msgs: None,
        };

        proposal.validate()?;

        PROPOSALS.save(deps.storage, id, &proposal)?;

        // Attach bank message to send the deposit amount to the burn address
        let burn_address = deps.api.addr_validate(BURN_ADDRESS)?;
        let burn_msg = BankMsg::Send {
            to_address: burn_address.to_string(),
            amount: vec![Coin {
                denom: "ujmes".to_string(),
                amount: deposit_amount,
            }],
        };

        Ok(Response::new().add_message(burn_msg))
    }

    pub fn request_feature(
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        config: Config,
        period_info: PeriodInfoResponse,
        deposit_amount: Uint128,
        title: String,
        description: String,
        funding: Option<Funding>,
        feature: Feature,
    ) -> Result<Response, ContractError> {
        let msg = match feature {
            Feature::ArtistCurator { approved, duration } => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.artist_curator_addr.unwrap().to_string(),
                msg: to_binary(&ApproveCurator {
                    dao: info.sender.clone(),
                    approved,
                    duration,
                })?,
                funds: vec![],
            }),
        };

        let id = Proposal::next_id(deps.storage)?;
        let proposal = Proposal {
            id,
            dao: info.sender,
            title,
            description,
            prop_type: ProposalType::FeatureRequest(feature),
            coins_no: Uint128::zero(),
            coins_yes: Uint128::zero(),
            yes_voters: Vec::new(),
            no_voters: Vec::new(),
            deposit_amount,
            start_block: env.block.height, // used for voting coin lookup
            posting_start: period_info.current_posting_start,
            voting_start: period_info.current_voting_start,
            voting_end: period_info.current_voting_end,
            concluded: false,
            funding,
            msgs: Some(vec![msg]),
        };

        proposal.validate()?;

        PROPOSALS.save(deps.storage, id, &proposal)?;

        // Attach bank message to send the deposit amount to the burn address
        let burn_address = deps.api.addr_validate(BURN_ADDRESS)?;
        let burn_msg = BankMsg::Send {
            to_address: burn_address.to_string(),
            amount: vec![Coin {
                denom: "ujmes".to_string(),
                amount: deposit_amount,
            }],
        };

        Ok(Response::new().add_message(burn_msg))
    }

    pub fn improvement(
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        _config: Config,
        period_info: PeriodInfoResponse,
        deposit_amount: Uint128,
        title: String,
        description: String,
        funding: Option<Funding>,
        msgs: Vec<CosmosMsg>,
    ) -> Result<Response, ContractError> {
        let core_slots = CORE_SLOTS.load(deps.storage)?;

        // Only the CoreSlot DAO can submit proposals
        if core_slots.core_tech.map(|s| s.dao) != Some(info.sender.clone()) {
            return Err(ContractError::Unauthorized {});
        }

        let core_tech_dao = info.sender.clone();

        let id = Proposal::next_id(deps.storage)?;
        let proposal = Proposal {
            id,
            dao: core_tech_dao.clone(),
            title,
            description,
            prop_type: ProposalType::Improvement {},
            coins_no: Uint128::zero(),
            coins_yes: Uint128::zero(),
            yes_voters: Vec::new(),
            no_voters: Vec::new(),
            deposit_amount,
            start_block: env.block.height, // used for voting coin lookup
            posting_start: period_info.current_posting_start,
            voting_start: period_info.current_voting_start,
            voting_end: period_info.current_voting_end,
            concluded: false,
            funding,
            msgs: Some(msgs),
        };

        proposal.validate()?;

        PROPOSALS.save(deps.storage, id, &proposal)?;

        // Attach bank message to send the deposit amount to the burn address
        let burn_address = deps.api.addr_validate(BURN_ADDRESS)?;
        let burn_msg = BankMsg::Send {
            to_address: burn_address.to_string(),
            amount: vec![Coin {
                denom: "ujmes".to_string(),
                amount: deposit_amount,
            }],
        };

        Ok(Response::new().add_message(burn_msg))
    }

    pub fn core_slot(
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        _config: Config,
        period_info: PeriodInfoResponse,
        deposit_amount: Uint128,
        title: String,
        description: String,
        funding: Option<Funding>,
        slot: CoreSlot,
    ) -> Result<Response, ContractError> {
        let dao = info.sender.clone();

        let id = Proposal::next_id(deps.storage)?;
        let proposal = Proposal {
            id,
            dao: dao.clone(),
            title,
            description,
            prop_type: ProposalType::CoreSlot(slot.clone()),
            coins_no: Uint128::zero(),
            coins_yes: Uint128::zero(),
            yes_voters: Vec::new(),
            no_voters: Vec::new(),
            deposit_amount,
            start_block: env.block.height, // used for voting coin lookup
            posting_start: period_info.current_posting_start,
            voting_start: period_info.current_voting_start,
            voting_end: period_info.current_voting_end,
            concluded: false,
            funding,
            msgs: Some(vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::SetCoreSlot { proposal_id: id })?,
                funds: vec![],
            })]),
        };

        proposal.validate()?;

        PROPOSALS.save(deps.storage, id, &proposal)?;

        // Attach bank message to send the deposit amount to the burn address
        let burn_address = deps.api.addr_validate(BURN_ADDRESS)?;
        let burn_msg = BankMsg::Send {
            to_address: burn_address.to_string(),
            amount: vec![Coin {
                denom: "ujmes".to_string(),
                amount: deposit_amount,
            }],
        };

        Ok(Response::new().add_message(burn_msg))
    }

    pub fn vote(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        id: u64,
        vote: VoteOption,
    ) -> Result<Response, ContractError> {
        {
            let period_info = period_info(deps.as_ref(), env.clone())?;

            if period_info.current_period != ProposalPeriod::Voting {
                return Err(ContractError::NotVotingPeriod {});
            }

            let mut proposal = PROPOSALS.load(deps.storage, id)?;

            println!("\n\n proposal {:?}", proposal);
            if proposal.concluded {
                return Err(ContractError::ProposalAlreadyConcluded {});
            }

            if proposal.voting_end < env.block.time.seconds() {
                return Err(ContractError::ProposalVotingEnded {});
            }

            if proposal.yes_voters.contains(&info.sender)
                || proposal.no_voters.contains(&info.sender)
            {
                return Err(ContractError::UserAlreadyVoted {});
            }

            // Check users bjmes balance (voting coins)
            let bjmes_amount = deps
                .querier
                .query_balance(info.sender.to_string(), "bujmes")?;

            let vote_coins = bjmes_amount.amount;

            if vote_coins.is_zero() {
                return Err(ContractError::NoVoteCoins {});
            }

            if vote_coins < Uint128::from(MIN_VOTE_COINS) {
                return Err(ContractError::InsufficientVoteCoins {});
            }

            match vote {
                Yes {} => {
                    proposal.coins_yes = proposal.coins_yes.checked_add(vote_coins)?;
                    proposal.yes_voters.push(info.sender.clone());
                }
                No {} => {
                    proposal.coins_no = proposal.coins_no.checked_add(vote_coins)?;
                    proposal.no_voters.push(info.sender.clone());
                }
            };

            PROPOSALS.save(deps.storage, id, &proposal)?;

            Ok(Response::new())
        }
    }

    // Process funding requests and Execute attached msgs
    pub fn conclude(deps: DepsMut, env: Env, id: u64) -> Result<Response, ContractError> {
        let mut proposal = PROPOSALS.load(deps.storage, id)?;
        let config = CONFIG.load(deps.storage)?;

        if env.block.time.seconds() <= proposal.voting_end {
            return Err(ContractError::VotingPeriodNotEnded {});
        }

        if proposal.concluded {
            return Err(ContractError::ProposalAlreadyConcluded {});
        }

        proposal.concluded = true;

        PROPOSALS.save(deps.storage, id, &proposal)?;

        let mut msgs: Vec<CosmosMsg> = vec![];

        let mut winning_grants = WINNING_GRANTS.load(deps.storage)?;

        // Remove expired grants from winning grants
        winning_grants.retain(|grant| grant.expire_at_height >= env.clone().block.height);

        // On proposal success, add winning_grant, process funding proposal and execute attached msgs
        if proposal.status(
            &deps.querier,
            env.clone(),
            config.proposal_required_percentage,
        ) == ProposalStatus::SuccessConcluded
        {
            if proposal.msgs.is_some() {
                msgs.extend(proposal.msgs.unwrap());
            }

            if proposal.funding.is_some() {
                winning_grants.push(WinningGrant {
                    dao: proposal.dao.clone(),
                    amount: proposal.funding.clone().unwrap().amount,
                    expire_at_height: proposal.funding.clone().unwrap().duration_in_blocks
                        + &env.block.height,
                    yes_ratio: Decimal::from_ratio(
                        proposal.coins_yes,
                        proposal.coins_yes + proposal.coins_no,
                    ),
                });
            }
        }

        // Finally save winning grants after housekeeping and adding the new funding grant
        WINNING_GRANTS.save(deps.storage, &winning_grants)?;

        Ok(Response::new().add_messages(msgs))
    }

    pub fn resign_core_slot(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        slot: CoreSlot,
        note: String,
    ) -> Result<Response, ContractError> {
        let mut core_slots = CORE_SLOTS.load(deps.storage)?;

        match slot {
            CoreSlot::Brand {} => {
                if core_slots.brand.unwrap().dao != info.sender {
                    return Err(ContractError::Unauthorized {});
                }
                core_slots.brand = None;
            }
            CoreSlot::CoreTech {} => {
                if core_slots.core_tech.unwrap().dao != info.sender {
                    return Err(ContractError::Unauthorized {});
                }
                core_slots.core_tech = None;
            }
            CoreSlot::Creative {} => {
                if core_slots.creative.unwrap().dao != info.sender {
                    return Err(ContractError::Unauthorized {});
                }
                core_slots.creative = None;
            }
        }

        CORE_SLOTS.save(deps.storage, &core_slots)?;

        Ok(Response::new()
            .add_attribute("action", "resign_core_slot")
            .add_attribute("dao", info.sender.to_string())
            .add_attribute("slot", slot.to_string())
            .add_attribute("note", note))
    }

    pub fn revoke_core_slot(
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        _config: Config,
        period_info: PeriodInfoResponse,
        deposit_amount: Uint128,
        title: String,
        description: String,
        funding: Option<Funding>,
        revoke_slot: RevokeCoreSlot,
    ) -> Result<Response, ContractError> {
        let dao = info.sender.clone();

        let id = Proposal::next_id(deps.storage)?;
        let proposal = Proposal {
            id,
            dao: dao.clone(),
            title,
            description,
            prop_type: ProposalType::RevokeCoreSlot(revoke_slot),
            coins_no: Uint128::zero(),
            coins_yes: Uint128::zero(),
            yes_voters: Vec::new(),
            no_voters: Vec::new(),
            deposit_amount,
            start_block: env.block.height, // used for voting coin lookup
            posting_start: period_info.current_posting_start,
            voting_start: period_info.current_voting_start,
            voting_end: period_info.current_voting_end,
            concluded: false,
            funding,
            msgs: Some(vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::UnsetCoreSlot { proposal_id: id })?,
                funds: vec![],
            })]),
        };

        println!("\n\nproposal {:?}", proposal);

        proposal.validate()?;

        PROPOSALS.save(deps.storage, id, &proposal)?;

        // Attach bank message to send the deposit amount to the burn address
        let burn_address = deps.api.addr_validate(BURN_ADDRESS)?;
        let burn_msg = BankMsg::Send {
            to_address: burn_address.to_string(),
            amount: vec![Coin {
                denom: "ujmes".to_string(),
                amount: deposit_amount,
            }],
        };

        Ok(Response::new().add_message(burn_msg))
    }

    pub fn unset_core_slot(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        proposal_id: u64,
    ) -> Result<Response, ContractError> {
        // Only the governance contract itself can unset core slots
        if info.sender != env.contract.address {
            return Err(ContractError::Unauthorized {});
        }

        let proposal = PROPOSALS.load(deps.storage, proposal_id)?;

        let mut core_slots = CORE_SLOTS.load(deps.storage)?;

        match proposal.prop_type {
            ProposalType::RevokeCoreSlot(revoke_slot) => match revoke_slot {
                RevokeCoreSlot { slot, dao } => match slot {
                    CoreSlot::CoreTech {} => {
                        if core_slots.core_tech.unwrap().dao != dao {
                            return Err(ContractError::WrongDao {});
                        }
                        core_slots.core_tech = None;
                    }
                    CoreSlot::Brand {} => {
                        if core_slots.brand.unwrap().dao != dao {
                            return Err(ContractError::WrongDao {});
                        }
                        core_slots.brand = None;
                    }
                    CoreSlot::Creative {} => {
                        if core_slots.creative.unwrap().dao != dao {
                            return Err(ContractError::WrongDao {});
                        }
                        core_slots.creative = None;
                    }
                },
            },
            _ => {
                return Err(ContractError::ProposalNotValid {
                    error: "Invalid RevokeCoreSlot proposal".into(),
                });
            }
        }

        CORE_SLOTS.save(deps.storage, &core_slots)?;

        Ok(Response::new())
    }
    pub fn set_core_slot(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        proposal_id: u64,
    ) -> Result<Response, ContractError> {
        // Only the governance contract itself can set core slots
        if info.sender != env.contract.address {
            return Err(ContractError::Unauthorized {});
        }

        let proposal = PROPOSALS.load(deps.storage, proposal_id)?;

        let dao = deps.api.addr_validate(&proposal.dao.to_string())?;

        let yes_ratio =
            Decimal::from_ratio(proposal.coins_yes, proposal.coins_yes + proposal.coins_no);

        let proposal_voting_end = proposal.voting_end;

        let some_slot_vote_result = Some(SlotVoteResult {
            dao: dao.clone(),
            yes_ratio,
            proposal_voting_end,
        });

        let mut core_slots = CORE_SLOTS.load(deps.storage)?;

        // A DAO can only hold one core slot at a time
        if Some(dao.clone()) == core_slots.brand.as_ref().map(|s| s.dao.clone())
            || Some(dao.clone()) == core_slots.core_tech.as_ref().map(|s| s.dao.clone())
            || Some(dao.clone()) == core_slots.creative.as_ref().map(|s| s.dao.clone())
        {
            // We don't return an error because we want the proposal to be marked as concluded
            return Ok(Response::new().add_attributes(vec![
                ("action", "set_core_slot"),
                ("proposal_id", &proposal_id.to_string()),
                ("dao", &proposal.dao.to_string()),
                ("error", "dao already holds a core slot"),
            ]));
        }

        let result: String;

        fn winning_core_slot(
            current_slot: SlotVoteResult,
            new_slot: SlotVoteResult,
        ) -> (Option<SlotVoteResult>, String) {
            let result: String;

            if new_slot.proposal_voting_end > current_slot.proposal_voting_end {
                result = "claimed core slot from previous period slot vote result".to_string();
                (Some(new_slot), result)
            } else if new_slot.proposal_voting_end == current_slot.proposal_voting_end {
                if new_slot.yes_ratio > current_slot.yes_ratio {
                    result =
                    "claimed core slot from current period slot vote result with smaller yes_ratio".to_string();
                    (Some(new_slot), result)
                } else {
                    result = "error: slot vote result with larger yes_ratio exists".to_string();
                    (Some(current_slot), result)
                }
            } else {
                // the remaining arm of the condition: new_slot.proposal_voting_end < current_slot.proposal_voting_end
                result = "error: proposal is older than current slot vote result".to_string();
                (Some(current_slot), result)
            }
        }

        match proposal.prop_type {
            ProposalType::CoreSlot(CoreSlot::Brand {}) => {
                if core_slots.brand.is_none() {
                    core_slots.brand = some_slot_vote_result;
                    result = "claimed empty core slot".to_string();
                } else {
                    (core_slots.brand, result) = winning_core_slot(
                        core_slots.brand.unwrap(),
                        some_slot_vote_result.unwrap(),
                    );
                }
            }
            ProposalType::CoreSlot(CoreSlot::Creative {}) => {
                if core_slots.creative.is_none() {
                    core_slots.creative = some_slot_vote_result;
                    result = "claimed empty core slot".to_string();
                } else {
                    (core_slots.creative, result) = winning_core_slot(
                        core_slots.creative.unwrap(),
                        some_slot_vote_result.unwrap(),
                    );
                }
            }
            ProposalType::CoreSlot(CoreSlot::CoreTech {}) => {
                if core_slots.core_tech.is_none() {
                    core_slots.core_tech = some_slot_vote_result;
                    result = "claimed empty core slot".to_string();
                } else {
                    (core_slots.core_tech, result) = winning_core_slot(
                        core_slots.core_tech.unwrap(),
                        some_slot_vote_result.unwrap(),
                    );
                }
            }
            _ => {
                return Err(ContractError::InvalidProposalType {});
            }
        }

        CORE_SLOTS.save(deps.storage, &core_slots)?;

        println!("\n\n core_slots {:#?}", core_slots);

        Ok(Response::new().add_attributes(vec![
            ("action", "set_core_slot"),
            ("proposal_id", &proposal_id.to_string()),
            ("dao", &proposal.dao.to_string()),
            // ("proposal_type", &proposal.prop_type.to_string()),
            ("yes_ratio", &yes_ratio.to_string()),
            ("proposal_voting_end", &proposal_voting_end.to_string()),
            ("result", &result),
        ]))
    }

    // One time setup function
    pub fn set_contract(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        artist_curator: String,
        identityservice: String,
    ) -> Result<Response, ContractError> {
        let mut config = CONFIG.load(deps.storage)?;
        println!("\n\n config {:?}", config);

        if config.owner.is_none() || info.sender != config.owner.unwrap() {
            return Err(ContractError::Unauthorized {});
        }

        let artist_curator_addr = deps.api.addr_validate(&artist_curator)?;
        let identityservice_addr = deps.api.addr_validate(&identityservice)?;

        config.artist_curator_addr = Some(artist_curator_addr);
        config.identityservice_addr = Some(identityservice_addr);

        // Disables calling this fn a second time
        config.owner = None;

        CONFIG.save(deps.storage, &config)?;

        Ok(Response::new())
    }
}

mod query {
    use std::ops::Sub;

    use cosmwasm_std::Order;
    use cw_storage_plus::Bound;

    use crate::msg::{
        CoreSlotsResponse, PeriodInfoResponse, ProposalPeriod, ProposalResponse, ProposalsResponse,
        WinningGrantsResponse,
    };
    use crate::state::{PROPOSALS, PROPOSAL_COUNT};

    use super::*;

    pub fn period_info(deps: Deps, env: Env) -> StdResult<PeriodInfoResponse> {
        let config = CONFIG.load(deps.storage)?;

        let now = env.block.time.seconds();

        let time_delta = now.sub(config.period_start_epoch);

        let full_cycle = config
            .posting_period_length
            .checked_add(config.voting_period_length)
            .unwrap();

        let time_in_cycle = time_delta % full_cycle;

        let current_period = if time_in_cycle > config.posting_period_length {
            ProposalPeriod::Voting
        } else {
            ProposalPeriod::Posting
        };

        let current_posting_start = now - time_in_cycle;
        let current_voting_start = current_posting_start + config.posting_period_length;
        let current_voting_end = current_voting_start + config.voting_period_length;

        let next_posting_start = current_posting_start + full_cycle;
        let next_voting_start = current_voting_start + full_cycle;

        Ok(PeriodInfoResponse {
            current_block: env.block.height,
            current_period,
            current_time_in_cycle: time_in_cycle,
            current_posting_start,
            current_voting_start,
            current_voting_end,
            next_posting_start,
            next_voting_start,
            posting_period_length: config.posting_period_length,
            voting_period_length: config.voting_period_length,
            cycle_length: config.posting_period_length + config.voting_period_length,
        })
    }

    pub fn core_slots(deps: Deps, _env: Env) -> StdResult<CoreSlotsResponse> {
        let core_slots = CORE_SLOTS.load(deps.storage)?;
        Ok(CoreSlotsResponse {
            brand: core_slots.brand,
            creative: core_slots.creative,
            core_tech: core_slots.core_tech,
        })
    }

    pub fn winning_grants(deps: Deps, _env: Env) -> StdResult<WinningGrantsResponse> {
        let winning_grants = WINNING_GRANTS.load(deps.storage)?;
        Ok(WinningGrantsResponse { winning_grants })
    }

    pub fn proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
        let proposal = PROPOSALS.load(deps.storage, id)?;
        let config = CONFIG.load(deps.storage)?;

        Ok(ProposalResponse {
            id: proposal.id,
            dao: proposal.dao.clone(),
            title: proposal.title.clone(),
            description: proposal.description.clone(),
            prop_type: proposal.prop_type.clone(),
            coins_yes: proposal.coins_yes,
            coins_no: proposal.coins_no,
            yes_voters: proposal.yes_voters.clone(),
            no_voters: proposal.no_voters.clone(),
            deposit_amount: proposal.deposit_amount,
            start_block: proposal.start_block,
            posting_start: proposal.posting_start,
            voting_start: proposal.voting_start,
            voting_end: proposal.voting_end,
            concluded: proposal.concluded,
            status: proposal.status(
                &deps.querier,
                env.clone(),
                config.proposal_required_percentage,
            ),
        })
    }

    pub fn proposals(
        deps: Deps,
        env: Env,
        start: Option<u64>,
        limit: Option<u32>,
    ) -> StdResult<ProposalsResponse> {
        let proposal_count = PROPOSAL_COUNT.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;

        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start.map(|start| Bound::inclusive(start));

        let proposals = PROPOSALS
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (_, proposal) = item?;
                Ok(ProposalResponse {
                    id: proposal.id,
                    dao: proposal.dao.clone(),
                    title: proposal.title.clone(),
                    description: proposal.description.clone(),
                    prop_type: proposal.prop_type.clone(),
                    coins_yes: proposal.coins_yes,
                    coins_no: proposal.coins_no,
                    yes_voters: proposal.yes_voters.clone(),
                    no_voters: proposal.no_voters.clone(),
                    deposit_amount: proposal.deposit_amount,
                    start_block: proposal.start_block,
                    posting_start: proposal.posting_start,
                    voting_start: proposal.voting_start,
                    voting_end: proposal.voting_end,
                    concluded: proposal.concluded,
                    status: proposal.status(
                        &deps.querier,
                        env.clone(),
                        config.proposal_required_percentage,
                    ),
                })
            })
            .collect::<StdResult<Vec<_>>>()?;

        Ok(ProposalsResponse {
            proposal_count,
            proposals,
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use std::vec;

//     use cosmwasm_std::testing::mock_env;
//     use cosmwasm_std::testing::MockApi;
//     use cosmwasm_std::testing::MockStorage;
//     use cosmwasm_std::Addr;
//     use cosmwasm_std::Timestamp;
//     use cosmwasm_std::Uint128;
//     use cw20::BalanceResponse;
//     use cw20::Cw20ExecuteMsg;
//     use cw_multi_test::AppBuilder;
//     use cw_multi_test::BankKeeper;
//     use cw_multi_test::{ContractWrapper, Executor};

//     use bjmes_token::contract::execute as bjmes_execute;
//     use bjmes_token::contract::instantiate as bjmes_instantiate;
//     use bjmes_token::contract::query as bjmes_query;
//     use bjmes_token::msg::ExecuteMsg as BjmesExecuteMsg;
//     use bjmes_token::msg::InstantiateMsg as BjmesInstantiateMsg;
//     use bjmes_token::msg::QueryMsg as BjmesQueryMsg;

//     // use artist_curator::contract::execute as artist_curator_execute;
//     // use artist_curator::contract::instantiate as artist_curator_instantiate;
//     // use artist_curator::contract::query as artist_curator_query;
//     // use artist_curator::msg::ExecuteMsg as ArtistCuratorExecuteMsg;
//     // use artist_curator::msg::InstantiateMsg as ArtistCuratorInstantiateMsg;
//     // use artist_curator::msg::QueryMsg as ArtistCuratorQueryMsg;

//     use crate::state::ProposalStatus;
//     use crate::state::ProposalType;
//     use crate::state::VoteOption::{No, Yes};

//     use crate::msg::*;

//     use super::*;

//     const PROPOSAL_REQUIRED_DEPOSIT: u128 = 1000;

//     #[test]
//     fn text_proposal() {
//         let mut env = mock_env();
//         env.block.time = Timestamp::from_seconds(1660000010);
//         let api = MockApi::default();
//         let bank = BankKeeper::new();
//         let storage = MockStorage::new();

//         let mut app = AppBuilder::new()
//             .with_api(api)
//             .with_block(env.block)
//             .with_bank(bank)
//             .with_storage(storage)
//             .build(|_, _, _| {});

//         let owner = Addr::unchecked("owner");
//         let user1 = Addr::unchecked("user1");
//         let user2 = Addr::unchecked("user2");

//         // // Instantiate artist curator contract
//         // let artist_curator_code = ContractWrapper::new(
//         //     artist_curator_execute,
//         //     artist_curator_instantiate,
//         //     artist_curator_query,
//         // );
//         // let artist_curator_code_id = app.store_code(Box::new(artist_curator_code));

//         // let artist_curator_instance = app
//         //     .instantiate_contract(
//         //         artist_curator_code_id,
//         //         owner,
//         //         &ArtistCuratorInstantiateMsg {},
//         //         &[],
//         //         "bonded JMES Contract",
//         //         None,
//         //     )
//         //     .unwrap();

//         // Instantiate bonded JMES cw20 contract
//         let bjmes_code = ContractWrapper::new(bjmes_execute, bjmes_instantiate, bjmes_query);
//         let bjmes_code_id = app.store_code(Box::new(bjmes_code));

//         let bjmes_instance = app
//             .instantiate_contract(
//                 bjmes_code_id,
//                 owner,
//                 &BjmesInstantiateMsg {
//                     name: "bonded JMES".to_string(),
//                     symbol: "bjmes".to_string(),
//                     decimals: 10,
//                     initial_balances: vec![],
//                     marketing: None,
//                     mint: None,
//                 },
//                 &[],
//                 "bonded JMES Contract",
//                 None,
//             )
//             .unwrap();

//         // Mint bJMES token
//         let _mint_resp = app
//             .execute_contract(
//                 user1.clone(),
//                 bjmes_instance.clone(),
//                 &BjmesExecuteMsg::Mint {
//                     recipient: user1.clone().to_string(),
//                     amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT * 2),
//                 },
//                 &[],
//             )
//             .unwrap();

//         let _mint_resp = app
//             .execute_contract(
//                 user2.clone(),
//                 bjmes_instance.clone(),
//                 &BjmesExecuteMsg::Mint {
//                     recipient: user2.clone().to_string(),
//                     amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT - 50u128),
//                 },
//                 &[],
//             )
//             .unwrap();

//         // Query bJMES token balance
//         let msg = BjmesQueryMsg::Balance {
//             address: user1.clone().to_string(),
//         };
//         let resp: StdResult<BalanceResponse> =
//             app.wrap().query_wasm_smart(bjmes_instance.clone(), &msg);

//         assert_eq!(
//             resp.unwrap().balance,
//             Uint128::from(PROPOSAL_REQUIRED_DEPOSIT * 2)
//         );

//         // Instantiate governance contract
//         let governance_code = ContractWrapper::new(execute, instantiate, query);
//         let governance_code_id = app.store_code(Box::new(governance_code));

//         let governance_instance = app
//             .instantiate_contract(
//                 governance_code_id,
//                 Addr::unchecked("owner"),
//                 &InstantiateMsg {
//                     bjmes_token_addr: bjmes_instance.clone().to_string(),
//                     artist_curator_addr: None,
//                     proposal_required_deposit: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
//                     proposal_required_percentage: 51,
//                     period_start_epoch: 1660000000,
//                     posting_period_length: 300000,
//                     voting_period_length: 606864,
//                 },
//                 &[],
//                 "Governance Contract",
//                 None,
//             )
//             .unwrap();

//         // Query empty proposals
//         let resp: ProposalsResponse = app
//             .wrap()
//             .query_wasm_smart(
//                 governance_instance.clone(),
//                 &QueryMsg::Proposals {
//                     start: None,
//                     limit: None,
//                 },
//             )
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalsResponse {
//                 proposal_count: 0,
//                 proposals: vec![]
//             }
//         );

//         // Query contract config
//         let config: Config = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Config {})
//             .unwrap();

//         assert_eq!(config.bjmes_token_addr, bjmes_instance);
//         assert_eq!(
//             config.proposal_required_deposit,
//             Uint128::from(config.proposal_required_deposit)
//         );

//         // Query PeriodInfo: Posting
//         let res: PeriodInfoResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::PeriodInfo {})
//             .unwrap();

//         assert_eq!(res.current_period, ProposalPeriod::Posting);
//         assert_eq!(res.current_time_in_cycle, 10);

//         // Skip period from Posting to Voting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.posting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Query PeriodInfo: Voting
//         let res: PeriodInfoResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::PeriodInfo {})
//             .unwrap();

//         assert_eq!(res.current_period, ProposalPeriod::Voting);
//         assert_eq!(res.current_time_in_cycle, 10 + config.posting_period_length);

//         // Test proposal in voting period
//         let text_proposal_msg = Cw20ExecuteMsg::Send {
//             contract: governance_instance.to_string(),
//             msg: to_binary(&Cw20HookMsg::TextProposal {
//                 title: String::from("Text"),
//                 description: String::from("Proposal"),
//             })
//             .unwrap(),
//             amount: Uint128::from(config.proposal_required_deposit),
//         };

//         let err = app
//             .execute_contract(
//                 user1.clone(),
//                 bjmes_instance.clone(),
//                 &text_proposal_msg,
//                 &[],
//             )
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "NotPostingPeriod");

//         // Skip period from Voting to Posting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.voting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Test proposal with insufficient deposit amount
//         let text_proposal_msg = Cw20ExecuteMsg::Send {
//             contract: governance_instance.to_string(),
//             msg: to_binary(&Cw20HookMsg::TextProposal {
//                 title: String::from("Text"),
//                 description: String::from("Proposal"),
//             })
//             .unwrap(),
//             amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT - 1u128),
//         };

//         let err = app
//             .execute_contract(
//                 user1.clone(),
//                 bjmes_instance.clone(),
//                 &text_proposal_msg,
//                 &[],
//             )
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "Insufficient token deposit!");

//         // Test valid proposal submission
//         let text_proposal_msg = Cw20ExecuteMsg::Send {
//             contract: governance_instance.to_string(),
//             msg: to_binary(&Cw20HookMsg::TextProposal {
//                 title: String::from("Text"),
//                 description: String::from("Proposal"),
//             })
//             .unwrap(),
//             amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
//         };

//         let _resp = app
//             .execute_contract(
//                 user1.clone(),
//                 bjmes_instance.clone(),
//                 &text_proposal_msg,
//                 &[],
//             )
//             .unwrap();

//         let resp: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();
//         assert_eq!(
//             resp,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Text".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::Text {},
//                 coins_yes: Uint128::zero(),
//                 coins_no: Uint128::zero(),
//                 yes_voters: vec![],
//                 no_voters: vec![],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: false,
//                 status: ProposalStatus::Posted
//             }
//         );

//         let resp: ProposalsResponse = app
//             .wrap()
//             .query_wasm_smart(
//                 governance_instance.clone(),
//                 &QueryMsg::Proposals {
//                     start: None,
//                     limit: None,
//                 },
//             )
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalsResponse {
//                 proposal_count: 1,
//                 proposals: vec![ProposalResponse {
//                     id: 1,
//                     dao: user1.clone(),
//                     title: "Text".to_string(),
//                     description: "Proposal".to_string(),
//                     prop_type: ProposalType::Text {},
//                     coins_yes: Uint128::zero(),
//                     coins_no: Uint128::zero(),
//                     yes_voters: vec![],
//                     no_voters: vec![],
//                     deposit_amount: Uint128::from(1000u128),
//                     start_block: 132345,
//                     posting_start: 1660906864,
//                     voting_start: 1661206864,
//                     voting_end: 1661813728,
//                     concluded: false,
//                     status: ProposalStatus::Posted
//                 }]
//             }
//         );

//         // Query bJMES token balance after proposal submission
//         let msg = BjmesQueryMsg::Balance {
//             address: user1.clone().to_string(),
//         };
//         let resp: StdResult<BalanceResponse> =
//             app.wrap().query_wasm_smart(bjmes_instance.clone(), &msg);

//         assert_eq!(
//             resp.unwrap().balance,
//             Uint128::from(config.proposal_required_deposit)
//         );

//         // TODO test vote with no coins

//         // Test proposal vote in posting period
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: Yes };

//         let err = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "NotVotingPeriod");

//         // Skip period from Posting to Voting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.posting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Query PeriodInfo: Voting
//         let res: PeriodInfoResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::PeriodInfo {})
//             .unwrap();

//         assert_eq!(res.current_period, ProposalPeriod::Voting);
//         assert_eq!(res.current_time_in_cycle, 10 + config.posting_period_length);

//         // Test proposal yes vote
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: Yes };

//         let _resp = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap();

//         let resp: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Text".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::Text {},
//                 coins_yes: Uint128::from(1000u128),
//                 coins_no: Uint128::zero(),
//                 yes_voters: vec![user1.clone()],
//                 no_voters: vec![],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: false,
//                 status: ProposalStatus::Voting
//             }
//         );

//         // Test proposal yes vote a second time
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: Yes };

//         let err = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "User already voted!");

//         // Test proposal no vote
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: No };

//         let _resp = app
//             .execute_contract(user2.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap();

//         let resp: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Text".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::Text {},
//                 coins_yes: Uint128::from(1000u128),
//                 coins_no: Uint128::from(950u128),
//                 yes_voters: vec![user1.clone()],
//                 no_voters: vec![user2.clone()],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: false,
//                 status: ProposalStatus::Voting
//             }
//         );

//         // Test proposal no vote a second time
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: No };

//         let err = app
//             .execute_contract(user2.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "User already voted!");

//         // Test conclude proposal still in voting period
//         let msg = ExecuteMsg::Conclude { id: 1 };

//         let err = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "VotingPeriodNotEnded");

//         // Skip period from Voting to Posting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.voting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Test conclude passing proposal
//         let msg = ExecuteMsg::Conclude { id: 1 };

//         let _resp = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &msg, &[])
//             .unwrap();

//         let resp_concluded: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();
//         println!("\n\n_resp {:?}", _resp);
//         assert_eq!(
//             resp_concluded,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Text".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::Text {},
//                 coins_yes: Uint128::from(1000u128),
//                 coins_no: Uint128::from(950u128),
//                 yes_voters: vec![user1.clone()],
//                 no_voters: vec![user2.clone()],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: true,
//                 status: ProposalStatus::SuccessConcluded
//             }
//         );

//         // TODO test expiredconcluded proposal

//         // Query bJMES token balance after proposal conclusion
//         let msg = BjmesQueryMsg::Balance {
//             address: user1.clone().to_string(),
//         };
//         let resp: StdResult<BalanceResponse> =
//             app.wrap().query_wasm_smart(bjmes_instance.clone(), &msg);

//         assert_eq!(
//             resp.unwrap().balance,
//             Uint128::from(PROPOSAL_REQUIRED_DEPOSIT * 2)
//         );

//         // TODO conclude expired proposal
//     }
//     #[test]
//     fn request_feature() {
//         let mut env = mock_env();
//         env.block.time = Timestamp::from_seconds(1660000010);
//         let api = MockApi::default();
//         let bank = BankKeeper::new();
//         let storage = MockStorage::new();

//         let mut app = AppBuilder::new()
//             .with_api(api)
//             .with_block(env.block)
//             .with_bank(bank)
//             .with_storage(storage)
//             .build(|_, _, _| {});

//         let owner = Addr::unchecked("owner");
//         let user1 = Addr::unchecked("user1");
//         let user2 = Addr::unchecked("user2");

//         // Instantiate artist curator contract
//         // let artist_curator_code = ContractWrapper::new(
//         //     artist_curator_execute,
//         //     artist_curator_instantiate,
//         //     artist_curator_query,
//         // );
//         // let artist_curator_code_id = app.store_code(Box::new(artist_curator_code));

//         // let artist_curator_instance = app
//         //     .instantiate_contract(
//         //         artist_curator_code_id,
//         //         owner,
//         //         &ArtistCuratorInstantiateMsg {},
//         //         &[],
//         //         "bonded JMES Contract",
//         //         None,
//         //     )
//         //     .unwrap();

//         // Instantiate bonded JMES cw20 contract
//         let bjmes_code = ContractWrapper::new(bjmes_execute, bjmes_instantiate, bjmes_query);
//         let bjmes_code_id = app.store_code(Box::new(bjmes_code));

//         let bjmes_instance = app
//             .instantiate_contract(
//                 bjmes_code_id,
//                 owner,
//                 &BjmesInstantiateMsg {
//                     name: "bonded JMES".to_string(),
//                     symbol: "bjmes".to_string(),
//                     decimals: 10,
//                     initial_balances: vec![],
//                     marketing: None,
//                     mint: None,
//                 },
//                 &[],
//                 "bonded JMES Contract",
//                 None,
//             )
//             .unwrap();

//         // Mint bJMES token
//         let _mint_resp = app
//             .execute_contract(
//                 user1.clone(),
//                 bjmes_instance.clone(),
//                 &BjmesExecuteMsg::Mint {
//                     recipient: user1.clone().to_string(),
//                     amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT * 2),
//                 },
//                 &[],
//             )
//             .unwrap();

//         let _mint_resp = app
//             .execute_contract(
//                 user2.clone(),
//                 bjmes_instance.clone(),
//                 &BjmesExecuteMsg::Mint {
//                     recipient: user2.clone().to_string(),
//                     amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT - 50u128),
//                 },
//                 &[],
//             )
//             .unwrap();

//         // Query bJMES token balance
//         let msg = BjmesQueryMsg::Balance {
//             address: user1.clone().to_string(),
//         };
//         let resp: StdResult<BalanceResponse> =
//             app.wrap().query_wasm_smart(bjmes_instance.clone(), &msg);

//         assert_eq!(
//             resp.unwrap().balance,
//             Uint128::from(PROPOSAL_REQUIRED_DEPOSIT * 2)
//         );

//         // Instantiate governance contract
//         let governance_code = ContractWrapper::new(execute, instantiate, query);
//         let governance_code_id = app.store_code(Box::new(governance_code));

//         let governance_instance = app
//             .instantiate_contract(
//                 governance_code_id,
//                 Addr::unchecked("owner"),
//                 &InstantiateMsg {
//                     bjmes_token_addr: bjmes_instance.clone().to_string(),
//                     artist_curator_addr: bjmes_instance.clone().to_string(), // TODO replace with artist_curator addr
//                     proposal_required_deposit: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
//                     proposal_required_percentage: 51,
//                     period_start_epoch: 1660000000,
//                     posting_period_length: 300000,
//                     voting_period_length: 606864,
//                 },
//                 &[],
//                 "Governance Contract",
//                 None,
//             )
//             .unwrap();

//         // Query empty proposals
//         let resp: ProposalsResponse = app
//             .wrap()
//             .query_wasm_smart(
//                 governance_instance.clone(),
//                 &QueryMsg::Proposals {
//                     start: None,
//                     limit: None,
//                 },
//             )
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalsResponse {
//                 proposal_count: 0,
//                 proposals: vec![]
//             }
//         );

//         // Query contract config
//         let config: Config = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Config {})
//             .unwrap();

//         assert_eq!(config.bjmes_token_addr, bjmes_instance);
//         assert_eq!(
//             config.proposal_required_deposit,
//             Uint128::from(config.proposal_required_deposit)
//         );

//         // Query PeriodInfo: Posting
//         let res: PeriodInfoResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::PeriodInfo {})
//             .unwrap();

//         assert_eq!(res.current_period, ProposalPeriod::Posting);
//         assert_eq!(res.current_time_in_cycle, 10);

//         // Skip period from Posting to Voting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.posting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Query PeriodInfo: Voting
//         let res: PeriodInfoResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::PeriodInfo {})
//             .unwrap();

//         assert_eq!(res.current_period, ProposalPeriod::Voting);
//         assert_eq!(res.current_time_in_cycle, 10 + config.posting_period_length);

//         // Skip period from Voting to Posting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.voting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Test valid proposal submission
//         let request_feature_msg = Cw20ExecuteMsg::Send {
//             contract: governance_instance.to_string(),
//             msg: to_binary(&Cw20HookMsg::RequestFeature {
//                 title: String::from("Artist Curator"),
//                 description: String::from("Proposal"),
//                 feature: Feature::ArtistCurator {
//                     approved: 2,
//                     duration: 300,
//                 },
//             })
//             .unwrap(),
//             amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
//         };

//         let _resp = app
//             .execute_contract(
//                 user1.clone(),
//                 bjmes_instance.clone(),
//                 &request_feature_msg,
//                 &[],
//             )
//             .unwrap();

//         let resp: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();
//         assert_eq!(
//             resp,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Artist Curator".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::FeatureRequest(ArtistCurator {
//                     approved: 2,
//                     duration: 300
//                 }),
//                 coins_yes: Uint128::zero(),
//                 coins_no: Uint128::zero(),
//                 yes_voters: vec![],
//                 no_voters: vec![],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: false,
//                 status: ProposalStatus::Posted
//             }
//         );

//         let resp: ProposalsResponse = app
//             .wrap()
//             .query_wasm_smart(
//                 governance_instance.clone(),
//                 &QueryMsg::Proposals {
//                     start: None,
//                     limit: None,
//                 },
//             )
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalsResponse {
//                 proposal_count: 1,
//                 proposals: vec![ProposalResponse {
//                     id: 1,
//                     dao: user1.clone(),
//                     title: "Artist Curator".to_string(),
//                     description: "Proposal".to_string(),
//                     prop_type: ProposalType::FeatureRequest(ArtistCurator {
//                         approved: 2,
//                         duration: 300
//                     }),
//                     coins_yes: Uint128::zero(),
//                     coins_no: Uint128::zero(),
//                     yes_voters: vec![],
//                     no_voters: vec![],
//                     deposit_amount: Uint128::from(1000u128),
//                     start_block: 132345,
//                     posting_start: 1660906864,
//                     voting_start: 1661206864,
//                     voting_end: 1661813728,
//                     concluded: false,
//                     status: ProposalStatus::Posted
//                 }]
//             }
//         );

//         // Query bJMES token balance after proposal submission
//         let msg = BjmesQueryMsg::Balance {
//             address: user1.clone().to_string(),
//         };
//         let resp: StdResult<BalanceResponse> =
//             app.wrap().query_wasm_smart(bjmes_instance.clone(), &msg);

//         assert_eq!(
//             resp.unwrap().balance,
//             Uint128::from(config.proposal_required_deposit)
//         );

//         // TODO test vote with no coins

//         // Test proposal vote in posting period
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: Yes };

//         let err = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "NotVotingPeriod");

//         // Skip period from Posting to Voting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.posting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Query PeriodInfo: Voting
//         let res: PeriodInfoResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::PeriodInfo {})
//             .unwrap();

//         assert_eq!(res.current_period, ProposalPeriod::Voting);
//         assert_eq!(res.current_time_in_cycle, 10 + config.posting_period_length);

//         // Test proposal yes vote
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: Yes };

//         let _resp = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap();

//         let resp: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Artist Curator".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::FeatureRequest(ArtistCurator {
//                     approved: 2,
//                     duration: 300
//                 }),
//                 coins_yes: Uint128::from(1000u128),
//                 coins_no: Uint128::zero(),
//                 yes_voters: vec![user1.clone()],
//                 no_voters: vec![],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: false,
//                 status: ProposalStatus::Voting
//             }
//         );

//         // Test proposal yes vote a second time
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: Yes };

//         let err = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "User already voted!");

//         // Test proposal no vote
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: No };

//         let _resp = app
//             .execute_contract(user2.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap();

//         let resp: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();

//         assert_eq!(
//             resp,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Artist Curator".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::FeatureRequest(ArtistCurator {
//                     approved: 2,
//                     duration: 300
//                 }),
//                 coins_yes: Uint128::from(1000u128),
//                 coins_no: Uint128::from(950u128),
//                 yes_voters: vec![user1.clone()],
//                 no_voters: vec![user2.clone()],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: false,
//                 status: ProposalStatus::Voting
//             }
//         );

//         // Test proposal no vote a second time
//         let vote_msg = ExecuteMsg::Vote { id: 1, vote: No };

//         let err = app
//             .execute_contract(user2.clone(), governance_instance.clone(), &vote_msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "User already voted!");

//         // Test conclude proposal still in voting period
//         let msg = ExecuteMsg::Conclude { id: 1 };

//         let err = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &msg, &[])
//             .unwrap_err();

//         assert_eq!(err.root_cause().to_string(), "VotingPeriodNotEnded");

//         // Skip period from Voting to Posting
//         app.update_block(|mut block| {
//             block.time = block.time.plus_seconds(config.voting_period_length);
//             block.height += config.posting_period_length / 5;
//         });

//         // Test conclude passing proposal
//         let msg = ExecuteMsg::Conclude { id: 1 };

//         let _resp = app
//             .execute_contract(user1.clone(), governance_instance.clone(), &msg, &[])
//             .unwrap();

//         let resp_concluded: ProposalResponse = app
//             .wrap()
//             .query_wasm_smart(governance_instance.clone(), &QueryMsg::Proposal { id: 1 })
//             .unwrap();
//         println!("\n\n_resp {:?}", _resp);
//         assert_eq!(
//             resp_concluded,
//             ProposalResponse {
//                 id: 1,
//                 dao: user1.clone(),
//                 title: "Artist Curator".to_string(),
//                 description: "Proposal".to_string(),
//                 prop_type: ProposalType::FeatureRequest(ArtistCurator {
//                     approved: 2,
//                     duration: 300
//                 }),
//                 coins_yes: Uint128::from(1000u128),
//                 coins_no: Uint128::from(950u128),
//                 yes_voters: vec![user1.clone()],
//                 no_voters: vec![user2.clone()],
//                 deposit_amount: Uint128::from(1000u128),
//                 start_block: 132345,
//                 posting_start: 1660906864,
//                 voting_start: 1661206864,
//                 voting_end: 1661813728,
//                 concluded: true,
//                 status: ProposalStatus::SuccessConcluded
//             }
//         );

//         // TODO test expiredconcluded proposal

//         // Query bJMES token balance after proposal conclusion
//         let msg = BjmesQueryMsg::Balance {
//             address: user1.clone().to_string(),
//         };
//         let resp: StdResult<BalanceResponse> =
//             app.wrap().query_wasm_smart(bjmes_instance.clone(), &msg);

//         assert_eq!(
//             resp.unwrap().balance,
//             Uint128::from(PROPOSAL_REQUIRED_DEPOSIT * 2)
//         );

//         // TODO conclude expired proposal
//     }
// }

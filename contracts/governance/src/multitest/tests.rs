#![cfg(test)]

use std::sync::Arc;

use cosmwasm_std::{
    coins,
    testing::{mock_env, MockApi, MockStorage},
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Timestamp, Uint128, WasmMsg,
};
use cw_multi_test::{App, AppBuilder, AppResponse, BankKeeper, Executor};
use cw_utils::{Duration, Threshold};
use dao::multitest::contract::DaoContract;
use distribution::multitest::contract::DistributionContract;
use identityservice::multitest::contract::IdentityserviceContract;
use jmes::{msg::Voter, test_utils::get_attribute};

// use crate::error::ContractError;

use crate::{
    error::ContractError,
    msg::{
        CoreSlot, CoreSlotsResponse, ExecuteMsg, PeriodInfoResponse, ProposalMsg, ProposalPeriod,
        ProposalResponse, QueryMsg, RevokeCoreSlot,
    },
    state::{ProposalStatus, SlotVoteResult, VoteOption},
};

use super::contract::GovernanceContract;
use bjmes_token::multitest::contract::BjmesTokenContract;

const SECONDS_PER_BLOCK: u64 = 5;
const PROPOSAL_REQUIRED_DEPOSIT: u128 = 1000;
const EPOCH_START: u64 = 1_660_000_010;

const FUNDING_DURATION: u64 = 1000000u64;
const FUNDING_AMOUNT: u128 = 1000000u128;

const USER1_VOTING_COINS: u128 = 2000;
const USER2_VOTING_COINS: u128 = 3000;

const DISTRIBUTION_INIT_BALANCE: u128 = 10_000_000;
const GOVERNANCE_INIT_BALANCE: u128 = 100_000; // To test improvement proposal: BankMsg

fn mock_app() -> App {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);
    let api = MockApi::default();
    let storage = MockStorage::new();
    let bank: BankKeeper = BankKeeper::new();

    AppBuilder::new()
        .with_api(api)
        .with_block(env.block)
        .with_storage(storage)
        .with_bank(bank)
        .build(|_, _, _| {})
}

#[derive(Debug, Clone)]
struct Contracts {
    governance: GovernanceContract,
    bjmes_token: BjmesTokenContract,
    distribution: DistributionContract,
    identityservice: IdentityserviceContract,
}
fn instantiate_contracts(app: &mut App, user1: Addr, user2: Addr, owner: Addr) -> Contracts {
    // Instantiate needed contracts

    let bjmes_code_id = BjmesTokenContract::store_code(app);
    let bjmes_contract =
        BjmesTokenContract::instantiate(app, bjmes_code_id, &user1, "bjmes").unwrap();

    let governance_code_id = GovernanceContract::store_code(app);
    let governance_contract = GovernanceContract::instantiate(
        app,
        governance_code_id,
        &user1,
        "Counting contract",
        owner.clone().into(),
        bjmes_contract.addr().into(),
        None,
        Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
        50,
        0,
        20,
        20,
    )
    .unwrap();

    let dao_code_id = DaoContract::store_code(app);
    // DAO Contract is instantiate via identityservice_contract.register_dao()
    // For reference only:
    //
    // let dao_contract = DaoContract::instantiate(
    //     &mut app,
    //     dao_code_id,
    //     &user1,
    //     "dao",
    //     "mydao".to_string(),
    //     vec![
    //         Voter {
    //             addr: user1.clone().into(),
    //             weight: 1,
    //         },
    //         Voter {
    //             addr: user2.clone().into(),
    //             weight: 1,
    //         },
    //     ],
    //     Threshold::AbsoluteCount { weight: 2 },
    //     Duration::Time(100),
    // )
    // .unwrap();

    let identityservice_code_id = IdentityserviceContract::store_code(app);
    let identityservice_contract = IdentityserviceContract::instantiate(
        app,
        identityservice_code_id,
        &user1,
        "identityservice",
        governance_contract.addr().clone(),
        dao_code_id,
    )
    .unwrap();

    let distribution_code_id = DistributionContract::store_code(app);
    let distribution_contract = DistributionContract::instantiate(
        app,
        distribution_code_id,
        &user1,
        "distribution",
        governance_contract.addr().clone(),
        identityservice_contract.addr().clone(),
    )
    .unwrap();

    println!("\n\nbjmes_contract {:?}", bjmes_contract);
    println!("\n\ngovernance_contract {:?}", governance_contract);
    println!(
        "\n\nidentityservice_contract {:?}",
        identityservice_contract
    );
    println!("\n\ndistribution_contract {:?}", distribution_contract);

    // Set contract options
    governance_contract
        .set_contract(
            app,
            &owner,
            distribution_contract.addr().into(),
            "artist_curator".into(), // TODO instantiate artist_curator contract and use actual address
            identityservice_contract.addr().into(),
        )
        .unwrap();
    // Fund the distribution contract
    app.init_modules(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                distribution_contract.addr(),
                vec![Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(DISTRIBUTION_INIT_BALANCE),
                }],
            )
            .unwrap();

        router
            .bank
            .init_balance(
                storage,
                governance_contract.addr(),
                vec![Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(GOVERNANCE_INIT_BALANCE),
                }],
            )
            .unwrap();
    });

    // Mint bjmes tokens to user1 so it can vote
    let mint3 = bjmes_contract
        .mint(
            app,
            &user1,
            user1.clone().into(),
            Uint128::from(USER1_VOTING_COINS),
        )
        .unwrap();

    println!("\n\nmint3 {:?}", mint3);

    // Mint bjmes tokens to user2 so it can vote
    let mint3 = bjmes_contract
        .mint(
            app,
            &user2,
            user2.clone().into(),
            Uint128::from(USER2_VOTING_COINS),
        )
        .unwrap();

    println!("\n\nmint3 {:?}", mint3);

    // Produce a block to mine balances (used by BalanceAt)
    app.update_block(|mut block| {
        block.time = Timestamp::from_seconds(block.time.seconds() + SECONDS_PER_BLOCK);
        block.height += 1;
    });

    Contracts {
        governance: governance_contract,
        bjmes_token: bjmes_contract,
        distribution: distribution_contract,
        identityservice: identityservice_contract,
    }
}

fn create_dao(app: &mut App, contracts: Contracts, user1: Addr, user2: Addr) -> Addr {
    // Register dao identity with valid name

    let my_dao = contracts
        .identityservice
        .register_dao(
            app,
            &user1,
            "my_dao".to_string(),
            vec![
                Voter {
                    addr: user1.clone().into(),
                    weight: 1,
                },
                Voter {
                    addr: user2.clone().into(),
                    weight: 1,
                },
            ],
            Threshold::AbsoluteCount { weight: 2 },
            Duration::Time(1000),
        )
        .unwrap();

    let my_dao_addr = my_dao
        .events
        .into_iter()
        .find(|event| event.ty == "instantiate")
        .unwrap()
        .attributes
        .into_iter()
        .find(|attribute| attribute.key == "_contract_addr")
        .unwrap()
        .value;

    println!("\n\nmy_dao_addr {:?}", my_dao_addr);

    assert_eq!(my_dao_addr, "contract4");

    // Fund dao addr with JMES so it can send the deposit
    let send_tokens_res = app
        .send_tokens(
            contracts.distribution.addr().clone(),
            Addr::unchecked(my_dao_addr.clone()),
            &coins(PROPOSAL_REQUIRED_DEPOSIT, "uluna"),
        )
        .unwrap();

    println!("\n\n send_tokens_res {:?}", send_tokens_res);

    Addr::unchecked(my_dao_addr)
}

fn gov_vote_helper(
    app: &mut App,
    contracts: Contracts,
    user1: Addr,
    user1_vote: VoteOption,
    _user2: Addr,
    _user2_vote: VoteOption,
    proposal_id: u64,
) -> AppResponse {
    let period_info_posting = contracts.governance.query_period_info(app).unwrap();
    println!("\n\n period_info_posting {:?}", period_info_posting);
    assert_eq!(period_info_posting.current_period, ProposalPeriod::Posting);

    // Skip period from Posting to VotingBLOKSECNDS
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
    });

    let period_info_voting = contracts.governance.query_period_info(app).unwrap();
    println!("\n\n period_info_voting{:?}", period_info_voting);
    // // assert_eq!(
    // //     period_info_voting,
    // //     PeriodInfoResponse {
    // //         current_block: 12350,
    // //         current_period: ProposalPeriod::Voting,
    // //         current_time_in_cycle: 35,
    // //         current_posting_start: 1660000000,
    // //         current_voting_start: 1660000020,
    // //         current_voting_end: 1660000040,
    // //         next_posting_start: 1660000040,
    // //         next_voting_start: 1660000060,
    // //         posting_period_length: 20,
    // //         voting_period_length: 20,
    // //         cycle_length: 40
    // //     }
    // );

    // User1 votes yes to on the governance proposal to pass it

    let fund_proposal_vote = contracts
        .governance
        .vote(app, &user1, proposal_id, user1_vote)
        .unwrap();
    println!("\n\n fund_proposal_vote {:?}", fund_proposal_vote);

    let proposal_result = contracts
        .governance
        .query_proposal(app, proposal_id)
        .unwrap();
    println!("\n\n proposal_result {:?}", proposal_result);

    // Test that you can't conclude a proposal in the voting period
    let voting_not_ended_err = contracts
        .governance
        .conclude(app, &user1, proposal_id)
        .unwrap_err();
    assert_eq!(voting_not_ended_err, ContractError::VotingPeriodNotEnded {});

    // Skip period from Voting to Posting so we can conclude the prBLOoKSECNDS
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.voting_period_length);
        block.height += period_info_posting.voting_period_length / SECONDS_PER_BLOCK;
    });

    let period_info_posting2 = contracts.governance.query_period_info(app).unwrap();
    println!("\n\n period_info_posting2 {:?}", period_info_posting2);

    let conclude_proposal_result = contracts
        .governance
        .conclude(app, &user1, proposal_id)
        .unwrap();
    println!(
        "\n\n conclude_proposal_result {:?}",
        conclude_proposal_result
    );

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = contracts
        .governance
        .conclude(app, &user1, proposal_id)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );
    println!(
        "\n\n conclude2_proposal_result {:?}",
        conclude2_proposal_result
    );
    conclude_proposal_result
}

#[test]
fn set_core_slot_brand_then_revoke_fail_then_revoke() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        1,
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);

    // Check that my_dao_addr now has the CoreTech slot
    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    assert_eq!(core_slots.brand.unwrap().dao, my_dao_addr.clone());

    // Create a dao proposal to revoke from the DAO from the Brand slot
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::RevokeCoreSlot {
        title: "Remove Brand Dao".into(),
        description: "Leave it vacant".into(),
        revoke_slot: RevokeCoreSlot {
            slot: CoreSlot::Brand {},
            dao: my_dao_addr.clone().to_string(),
        },
    });

    // Failing Revoke Proposal

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    let revoke_result = gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::No,
        user2.clone(),
        VoteOption::No,
        2,
    );

    println!("\n\n revoke_result {:?}", revoke_result);

    let failing_proposal = contracts.governance.query_proposal(&mut app, 2).unwrap();
    assert_eq!(
        failing_proposal,
        ProposalResponse {
            id: 2,
            dao: my_dao_addr.clone(),
            title: "Remove Brand Dao".into(),
            description: "Leave it vacant".into(),
            prop_type: crate::state::ProposalType::RevokeCoreSlot(RevokeCoreSlot {
                slot: CoreSlot::Brand {},
                dao: my_dao_addr.clone().into(),
            }),
            coins_yes: Uint128::from(0u128),
            coins_no: Uint128::from(2000u128),
            yes_voters: vec![],
            no_voters: vec![user1.clone()],
            deposit_amount: Uint128::from(1000u128),
            start_block: 12354,
            posting_start: 1660000040,
            voting_start: 1660000060,
            voting_end: 1660000080,
            concluded: true,
            status: ProposalStatus::ExpiredConcluded
        }
    );

    println!("\n\n failing_proposal {:?}", failing_proposal);

    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    println!("\n\n core_slots {:?}", core_slots);
    assert_eq!(
        core_slots.brand,
        Some(SlotVoteResult {
            dao: my_dao_addr.clone(),
            yes_ratio: Decimal::percent(100),
            proposal_voting_end: 1660000040
        })
    );

    // Successful Revoke Proposal

    // Fund my_dao_addr so it can send the deposit
    app.send_tokens(
        contracts.distribution.addr().clone(),
        Addr::unchecked(my_dao_addr.clone()),
        &coins(PROPOSAL_REQUIRED_DEPOSIT, "uluna"),
    )
    .unwrap();

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    let revoke_result = gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        3,
    );

    println!("\n\n revoke_result {:?}", revoke_result);

    let success_proposal = contracts.governance.query_proposal(&mut app, 3).unwrap();
    assert_eq!(
        success_proposal,
        ProposalResponse {
            id: 3,
            dao: my_dao_addr.clone(),
            title: "Remove Brand Dao".into(),
            description: "Leave it vacant".into(),
            prop_type: crate::state::ProposalType::RevokeCoreSlot(RevokeCoreSlot {
                slot: CoreSlot::Brand {},
                dao: my_dao_addr.clone().into(),
            }),
            coins_yes: Uint128::from(2000u128),
            coins_no: Uint128::from(0u128),
            yes_voters: vec![user1.clone()],
            no_voters: vec![],
            deposit_amount: Uint128::from(1000u128),
            start_block: 12362,
            posting_start: 1660000080,
            voting_start: 1660000100,
            voting_end: 1660000120,
            concluded: true,
            status: ProposalStatus::SuccessConcluded
        }
    );

    println!("\n\n success_proposal {:?}", success_proposal);

    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    println!("\n\n core_slots {:?}", core_slots);
    assert_eq!(core_slots.brand, None);
}

#[test]
fn set_core_slot_creative_and_fail_setting_a_second_slot_for_the_same_dao() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        slot: CoreSlot::Creative {},
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        1,
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);

    // Check that my_dao_addr now has the Creative slot
    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    assert_eq!(core_slots.creative.unwrap().dao, my_dao_addr);

    // Fail to set a second slot for the same dao

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    let failed_core_slot_res = gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        2,
    );

    assert_eq!(
        get_attribute(&failed_core_slot_res, "error"),
        "dao already holds a core slot".to_string()
    );

    let failed_proposal = contracts.governance.query_proposal(&mut app, 2).unwrap();
    assert_eq!(failed_proposal.status, ProposalStatus::SuccessConcluded);
}

#[test]
fn set_core_slot_tech_and_resign() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        slot: CoreSlot::CoreTech {},
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        1,
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);

    // Check that my_dao_addr now has the CoreTech slot
    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    assert_eq!(core_slots.core_tech.unwrap().dao, my_dao_addr);

    // Create a dao proposal to resign from the CoreTech slot
    let proposal_msg = ExecuteMsg::ResignCoreSlot {
        slot: CoreSlot::CoreTech {},
        note: "Good bye!".into(),
    };

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr,
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();
    // TODO query core_slots and assert core_tech is empty
    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    println!("\n\n core_slots {:?}", core_slots);
    assert_eq!(core_slots.core_tech, None);
}

#[test]
fn set_core_slot_unauthorized() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    println!("\n\n contracts {:#?}", contracts);

    // Register user identity with valid name

    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    let set_core_slot_err = contracts
        .governance
        .set_core_slot(&mut app, &user1, 1)
        .unwrap_err();

    assert_eq!(set_core_slot_err, ContractError::Unauthorized {});
}

#[test]
fn improvement_bankmsg() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    println!("\n\n contracts {:#?}", contracts);

    // Register user identity with valid name

    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    // Only the CoreSlot DAO can submit an Improvement proposal
    // So we create a proposal to make my_dao_addr the CoreTech slot

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        slot: CoreSlot::CoreTech {},
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        1,
    );

    // Check that my_dao_addr now has the CoreTech slot
    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    assert_eq!(core_slots.core_tech.unwrap().dao, my_dao_addr);

    // Now create the Improvement proposal to send funds

    // Create a Dao Proposal for Governance Improvement Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Improvement {
        title: "Send funds".into(),
        description: "BankMsg".into(),
        msgs: vec![CosmosMsg::Bank(BankMsg::Send {
            to_address: user1.clone().into(),
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: Uint128::from(GOVERNANCE_INIT_BALANCE),
            }],
        })],
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr,
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    assert_eq!(
        app.wrap().query_all_balances(user1.clone()).unwrap(),
        vec![]
    );
    assert_eq!(
        app.wrap()
            .query_all_balances(contracts.governance.addr().clone())
            .unwrap(),
        coins(GOVERNANCE_INIT_BALANCE + PROPOSAL_REQUIRED_DEPOSIT, "uluna")
    );

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        2,
    );

    // Test that the funds were sent from governance to user1
    assert_eq!(
        app.wrap().query_all_balances(user1.clone()).unwrap(),
        coins(GOVERNANCE_INIT_BALANCE, "uluna")
    );
    assert_eq!(
        app.wrap()
            .query_all_balances(contracts.governance.addr().clone())
            .unwrap(),
        vec![]
    );
}

#[test]
fn improvement_bankmsg_failing() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    println!("\n\n contracts {:#?}", contracts);

    // Register user identity with valid name

    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    // Create, vote on and execute the dao proposal
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    // Only the CoreSlot DAO can submit an Improvement proposal
    // So we create a proposal to make my_dao_addr the CoreTech slot

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        slot: CoreSlot::CoreTech {},
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        1,
    );

    // Check that my_dao_addr now has the CoreTech slot
    let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
    assert_eq!(core_slots.core_tech.unwrap().dao, my_dao_addr);

    // Now create the Improvement proposal to send funds

    // Create a Dao Proposal for Governance Improvement Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Improvement {
        title: "Send funds".into(),
        description: "BankMsg".into(),
        msgs: vec![CosmosMsg::Bank(BankMsg::Send {
            to_address: user1.clone().into(),
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: Uint128::from(GOVERNANCE_INIT_BALANCE),
            }],
        })],
    });

    // Create, vote on and execute the dao proposal
    DaoContract::gov_proposal_helper(
        &mut app,
        my_dao_addr,
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    assert_eq!(
        app.wrap().query_all_balances(user1.clone()).unwrap(),
        vec![]
    );
    assert_eq!(
        app.wrap()
            .query_all_balances(contracts.governance.addr().clone())
            .unwrap(),
        coins(GOVERNANCE_INIT_BALANCE + PROPOSAL_REQUIRED_DEPOSIT, "uluna")
    );

    // Vote on and execute the governance proposal
    let proposal_result = gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::No,
        user2.clone(),
        VoteOption::No,
        2,
    );

    println!("\n\n final_proposal_result {:?}", proposal_result);

    // Test that deposit was forward to the distribution contract
    assert_eq!(
        app.wrap().query_all_balances(user1.clone()).unwrap(),
        vec![]
    );
    assert_eq!(
        app.wrap()
            .query_all_balances(contracts.governance.addr().clone())
            .unwrap(),
        coins(GOVERNANCE_INIT_BALANCE, "uluna")
    );
    assert_eq!(
        app.wrap()
            .query_all_balances(contracts.distribution.addr().clone())
            .unwrap(),
        coins(DISTRIBUTION_INIT_BALANCE, "uluna")
    );

    let final_proposal: ProposalResponse = app
        .wrap()
        .query_wasm_smart(contracts.governance.addr(), &QueryMsg::Proposal { id: 2 })
        .unwrap();
    println!("\n\n final_proposal {:#?}", final_proposal);
    assert_eq!(final_proposal.status, ProposalStatus::ExpiredConcluded);
}
#[test]
fn governance_funding_proposal_passing() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    // Instantiate needed contracts

    let bjmes_code_id = BjmesTokenContract::store_code(&mut app);
    let bjmes_contract =
        BjmesTokenContract::instantiate(&mut app, bjmes_code_id, &user1, "bjmes").unwrap();

    let governance_code_id = GovernanceContract::store_code(&mut app);
    let governance_contract = GovernanceContract::instantiate(
        &mut app,
        governance_code_id,
        &user1,
        "Counting contract",
        owner.clone().into(),
        bjmes_contract.addr().into(),
        None,
        Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
        50,
        0,
        20,
        20,
    )
    .unwrap();

    let dao_code_id = DaoContract::store_code(&mut app);
    // DAO Contract is instantiate via identityservice_contract.register_dao()
    // For reference only:
    //
    // let dao_contract = DaoContract::instantiate(
    //     &mut app,
    //     dao_code_id,
    //     &user1,
    //     "dao",
    //     "mydao".to_string(),
    //     vec![
    //         Voter {
    //             addr: user1.clone().into(),
    //             weight: 1,
    //         },
    //         Voter {
    //             addr: user2.clone().into(),
    //             weight: 1,
    //         },
    //     ],
    //     Threshold::AbsoluteCount { weight: 2 },
    //     Duration::Time(100),
    // )
    // .unwrap();

    let identityservice_code_id = IdentityserviceContract::store_code(&mut app);
    let identityservice_contract = IdentityserviceContract::instantiate(
        &mut app,
        identityservice_code_id,
        &user1,
        "identityservice",
        governance_contract.addr().clone(),
        dao_code_id,
    )
    .unwrap();

    let distribution_code_id = DistributionContract::store_code(&mut app);
    let distribution_contract = DistributionContract::instantiate(
        &mut app,
        distribution_code_id,
        &user1,
        "distribution",
        governance_contract.addr().clone(),
        identityservice_contract.addr().clone(),
    )
    .unwrap();

    println!("\n\nbjmes_contract {:?}", bjmes_contract);
    println!("\n\ngovernance_contract {:?}", governance_contract);
    println!(
        "\n\nidentityservice_contract {:?}",
        identityservice_contract
    );
    println!("\n\ndistribution_contract {:?}", distribution_contract);

    // Set contract options
    governance_contract
        .set_contract(
            &mut app,
            &owner,
            distribution_contract.addr().into(),
            "artist_curator".into(), // TODO instantiate artist_curator contract and use actual address
            identityservice_contract.addr().into(),
        )
        .unwrap();

    // Register user identity with valid name

    let user1_id = identityservice_contract
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    println!("\n\nuser1_id {:?}", user1_id);

    // Register dao identity with valid name

    let my_dao = identityservice_contract
        .register_dao(
            &mut app,
            &user1,
            "my_dao".to_string(),
            vec![
                Voter {
                    addr: user1.clone().into(),
                    weight: 1,
                },
                Voter {
                    addr: user2.clone().into(),
                    weight: 1,
                },
            ],
            Threshold::AbsoluteCount { weight: 2 },
            Duration::Time(1000),
        )
        .unwrap();

    let my_dao_addr = my_dao
        .events
        .into_iter()
        .find(|event| event.ty == "instantiate")
        .unwrap()
        .attributes
        .into_iter()
        .find(|attribute| attribute.key == "_contract_addr")
        .unwrap()
        .value;

    println!("\n\nmy_dao_addr {:?}", my_dao_addr);

    assert_eq!(my_dao_addr, "contract4");

    // Fund the distribution contract
    let fund_res = app.init_modules(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                distribution_contract.addr(),
                vec![Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(DISTRIBUTION_INIT_BALANCE),
                }],
            )
            .unwrap();
    });

    println!("\n\n fund_res {:?}", fund_res);

    // Fund my_dao_addr so it can send the deposit
    app.send_tokens(
        distribution_contract.addr().clone(),
        Addr::unchecked(my_dao_addr.clone()),
        &coins(PROPOSAL_REQUIRED_DEPOSIT, "uluna"),
    )
    .unwrap();

    // Mint bjmes tokens to user1 so it can vote
    let mint3 = bjmes_contract
        .mint(
            &mut app,
            &user1,
            user1.clone().into(),
            Uint128::from(USER1_VOTING_COINS),
        )
        .unwrap();

    println!("\n\nmint3 {:?}", mint3);

    // Mint bjmes tokens to user2 so it can vote
    let mint3 = bjmes_contract
        .mint(
            &mut app,
            &user2,
            user2.clone().into(),
            Uint128::from(USER2_VOTING_COINS),
        )
        .unwrap();

    println!("\n\nmint3 {:?}", mint3);

    // Create a Dao Proposal for Governance Funding

    // Governance Proposal Msg
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Funding {
        title: "Funding".to_string(),
        description: "Give me money".to_string(),
        duration: FUNDING_DURATION,
        amount: Uint128::from(FUNDING_AMOUNT),
    });

    let wasm_msg = WasmMsg::Execute {
        contract_addr: governance_contract.addr().clone().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        funds: coins(PROPOSAL_REQUIRED_DEPOSIT, "uluna"),
    };

    let submit_funding_proposal_result = DaoContract::propose(
        &mut app,
        &user1,
        &my_dao_addr,
        "Request Funding from Governance".into(),
        "Make us rich".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
    );

    println!(
        "\n\n submit_funding_proposal_result {:?}",
        submit_funding_proposal_result
    );

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoContract::vote(&mut app, &user2, &my_dao_addr, 1, cw3::Vote::Yes);
    println!("\n\n dao_vote2_result {:?}", dao_vote2_result);

    let dao_execute_result = DaoContract::execute(&mut app, &user1, &my_dao_addr, 1);
    println!("\n\n dao_execute_result {:?}", dao_execute_result);

    // Test after proposal execution the deposit is sent to the governance contract
    assert_eq!(
        app.wrap()
            .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
            .unwrap(),
        vec![]
    );

    let period_info_posting = governance_contract.query_period_info(&mut app).unwrap();
    println!("\n\n period_info_posting {:?}", period_info_posting);
    assert_eq!(period_info_posting.current_period, ProposalPeriod::Posting);

    // Skip period from Posting to VotingBLOKSECNDS
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
    });

    // assert_eq!(
    //     app.wrap()
    //         .query_all_balances(governance_contract.addr().clone())
    //         .unwrap(),
    //     coins(PROPOSAL_REQUIRED_DEPOSIT, "bjmes")
    // );

    let period_info_voting = governance_contract.query_period_info(&mut app).unwrap();
    println!("\n\n period_info_voting{:?}", period_info_voting);
    assert_eq!(
        period_info_voting,
        PeriodInfoResponse {
            current_block: 12349,
            current_period: ProposalPeriod::Voting,
            current_time_in_cycle: 30,
            current_posting_start: 1660000000,
            current_voting_start: 1660000020,
            current_voting_end: 1660000040,
            next_posting_start: 1660000040,
            next_voting_start: 1660000060,
            posting_period_length: 20,
            voting_period_length: 20,
            cycle_length: 40
        }
    );

    // User1 votes yes to on the governance proposal to pass it

    let fund_proposal_vote = governance_contract
        .vote(&mut app, &user1, 1, VoteOption::Yes)
        .unwrap();
    println!("\n\n fund_proposal_vote {:?}", fund_proposal_vote);

    let proposal_result = governance_contract.query_proposal(&mut app, 1).unwrap();
    println!("\n\n proposal_result {:?}", proposal_result);

    // Test that you can't conclude a proposal in the voting period
    let voting_not_ended_err = governance_contract
        .conclude(&mut app, &user1, 1)
        .unwrap_err();
    assert_eq!(voting_not_ended_err, ContractError::VotingPeriodNotEnded {});

    // Skip period from Voting to Posting so we can conclude the prBLOoKSECNDS
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.voting_period_length);
        block.height += period_info_posting.voting_period_length / SECONDS_PER_BLOCK;
    });

    let period_info_posting2 = governance_contract.query_period_info(&mut app).unwrap();
    println!("\n\n period_info_posting2 {:?}", period_info_posting2);

    let conclude_proposal_result = governance_contract.conclude(&mut app, &user1, 1).unwrap();
    println!(
        "\n\n conclude_proposal_result {:?}",
        conclude_proposal_result
    );

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = governance_contract
        .conclude(&mut app, &user1, 1)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );
    println!(
        "\n\n conclude2_proposal_result {:?}",
        conclude2_proposal_result
    );

    // Skip half the grant duration time to allow us to claim funds
    app.update_block(|mut block| {
        block.time = block.time.plus_seconds(FUNDING_DURATION / 2);
        block.height += FUNDING_DURATION / 2 / SECONDS_PER_BLOCK;
    });

    let claim_funds_result = distribution_contract.claim(&mut app, &user1, 1).unwrap();
    println!("\n\n claim_funds_result {:?}", claim_funds_result);

    assert_eq!(
        app.wrap()
            .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
            .unwrap(),
        coins(FUNDING_AMOUNT / 2 + PROPOSAL_REQUIRED_DEPOSIT, "uluna")
    );

    // Skip double the grant duration time to claim 100% of the funds
    app.update_block(|mut block| {
        block.time = block.time.plus_seconds(FUNDING_DURATION * 2);
        block.height += FUNDING_DURATION * 2 / SECONDS_PER_BLOCK;
    });

    let claim_funds_result = distribution_contract
        .claim(&mut app, &Addr::unchecked(my_dao_addr.clone()), 1)
        .unwrap();
    println!("\n\n claim_funds_result {:?}", claim_funds_result);

    assert_eq!(
        app.wrap()
            .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
            .unwrap(),
        coins(FUNDING_AMOUNT + PROPOSAL_REQUIRED_DEPOSIT, "uluna")
    );

    // Skip period from Posting to VotingBLOKSECNDS
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
    });

    let period_info_voting = governance_contract.query_period_info(&mut app).unwrap();
    println!("\n\n period_info_voting{:?}", period_info_voting);

    // Test that after conclusion, user2 can no longer vote on the proposal
    let post_conclusion_vote = governance_contract
        .vote(&mut app, &user2, 1, VoteOption::No)
        .unwrap_err();
    println!("\n\n post_conclusion_vote {:?}", post_conclusion_vote);

    assert_eq!(
        post_conclusion_vote,
        ContractError::ProposalAlreadyConcluded {}.into()
    );

    let post_conclusion_proposal = governance_contract.query_proposal(&mut app, 1).unwrap();
    assert_eq!(post_conclusion_proposal.coins_no, Uint128::zero());

    // Test that a failing proposal sends the deposit to the distribution contract and doesn't execute the msgs
}

#[test]
fn governance_funding_proposal_failing() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    // Instantiate needed contracts

    let bjmes_code_id = BjmesTokenContract::store_code(&mut app);
    let bjmes_contract =
        BjmesTokenContract::instantiate(&mut app, bjmes_code_id, &user1, "bjmes").unwrap();

    let governance_code_id = GovernanceContract::store_code(&mut app);
    let governance_contract = GovernanceContract::instantiate(
        &mut app,
        governance_code_id,
        &user1,
        "Counting contract",
        owner.clone().into(),
        bjmes_contract.addr().into(),
        None,
        Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
        50,
        0,
        20,
        20,
    )
    .unwrap();

    let dao_code_id = DaoContract::store_code(&mut app);
    // DAO Contract is instantiate via identityservice_contract.register_dao()
    // For reference only:
    //
    // let dao_contract = DaoContract::instantiate(
    //     &mut app,
    //     dao_code_id,
    //     &user1,
    //     "dao",
    //     "mydao".to_string(),
    //     vec![
    //         Voter {
    //             addr: user1.clone().into(),
    //             weight: 1,
    //         },
    //         Voter {
    //             addr: user2.clone().into(),
    //             weight: 1,
    //         },
    //     ],
    //     Threshold::AbsoluteCount { weight: 2 },
    //     Duration::Time(100),
    // )
    // .unwrap();

    let identityservice_code_id = IdentityserviceContract::store_code(&mut app);
    let identityservice_contract = IdentityserviceContract::instantiate(
        &mut app,
        identityservice_code_id,
        &user1,
        "identityservice",
        governance_contract.addr().clone(),
        dao_code_id,
    )
    .unwrap();

    let distribution_code_id = DistributionContract::store_code(&mut app);
    let distribution_contract = DistributionContract::instantiate(
        &mut app,
        distribution_code_id,
        &user1,
        "distribution",
        governance_contract.addr().clone(),
        identityservice_contract.addr().clone(),
    )
    .unwrap();

    println!("\n\nbjmes_contract {:?}", bjmes_contract);
    println!("\n\ngovernance_contract {:?}", governance_contract);
    println!(
        "\n\nidentityservice_contract {:?}",
        identityservice_contract
    );
    println!("\n\ndistribution_contract {:?}", distribution_contract);

    // Set contract options
    governance_contract
        .set_contract(
            &mut app,
            &owner,
            distribution_contract.addr().into(),
            "artist_curator".into(), // TODO instantiate artist_curator contract and use actual address
            identityservice_contract.addr().into(),
        )
        .unwrap();

    // Register user identity with valid name

    let user1_id = identityservice_contract
        .register_user(&mut app, &user1, "user1_id".to_string())
        .unwrap();

    println!("\n\nuser1_id {:?}", user1_id);

    // Register dao identity with valid name

    let my_dao = identityservice_contract
        .register_dao(
            &mut app,
            &user1,
            "my_dao".to_string(),
            vec![
                Voter {
                    addr: user1.clone().into(),
                    weight: 1,
                },
                Voter {
                    addr: user2.clone().into(),
                    weight: 1,
                },
            ],
            Threshold::AbsoluteCount { weight: 2 },
            Duration::Time(1000),
        )
        .unwrap();

    let my_dao_addr = my_dao
        .events
        .into_iter()
        .find(|event| event.ty == "instantiate")
        .unwrap()
        .attributes
        .into_iter()
        .find(|attribute| attribute.key == "_contract_addr")
        .unwrap()
        .value;

    println!("\n\nmy_dao_addr {:?}", my_dao_addr);

    assert_eq!(my_dao_addr, "contract4");

    // Fund the distribution contract
    let fund_res = app.init_modules(|router, _, storage| {
        router.bank.init_balance(
            storage,
            distribution_contract.addr(),
            vec![Coin {
                denom: "uluna".to_string(),
                amount: Uint128::from(DISTRIBUTION_INIT_BALANCE),
            }],
        )
    });

    println!("\n\n fund_res {:?}", fund_res);

    // Fund my_dao_addr so it can send the deposit
    app.send_tokens(
        distribution_contract.addr().clone(),
        Addr::unchecked(my_dao_addr.clone()),
        &coins(PROPOSAL_REQUIRED_DEPOSIT, "uluna"),
    )
    .unwrap();

    // Mint bjmes tokens to user1 so it can vote
    let mint3 = bjmes_contract
        .mint(
            &mut app,
            &user1,
            user1.clone().into(),
            Uint128::from(USER1_VOTING_COINS),
        )
        .unwrap();

    println!("\n\nmint3 {:?}", mint3);

    // Mint bjmes tokens to user2 so it can vote
    let mint3 = bjmes_contract
        .mint(
            &mut app,
            &user2,
            user2.clone().into(),
            Uint128::from(USER2_VOTING_COINS),
        )
        .unwrap();

    println!("\n\nmint3 {:?}", mint3);

    // Create a Dao Proposal for Governance Funding

    // Governance Proposal Msg
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Funding {
        title: "Funding".to_string(),
        description: "Give me money".to_string(),
        duration: FUNDING_DURATION,
        amount: Uint128::from(FUNDING_AMOUNT),
    });

    let wasm_msg = WasmMsg::Execute {
        contract_addr: governance_contract.addr().clone().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        funds: coins(PROPOSAL_REQUIRED_DEPOSIT, "uluna"),
    };

    let submit_funding_proposal_result = DaoContract::propose(
        &mut app,
        &user1,
        &my_dao_addr,
        "Request Funding from Governance".into(),
        "Make us rich".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
    );

    println!(
        "\n\n submit_funding_proposal_result {:?}",
        submit_funding_proposal_result
    );

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoContract::vote(&mut app, &user2, &my_dao_addr, 1, cw3::Vote::Yes);
    println!("\n\n dao_vote2_result {:?}", dao_vote2_result);

    let dao_execute_result = DaoContract::execute(&mut app, &user1, &my_dao_addr, 1);
    println!("\n\n dao_execute_result {:?}", dao_execute_result);

    let period_info_posting = governance_contract.query_period_info(&mut app).unwrap();
    println!("\n\n period_info_posting {:?}", period_info_posting);
    assert_eq!(period_info_posting.current_period, ProposalPeriod::Posting);

    // Skip period from Posting to VotingBLOKSECNDS
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
    });

    let period_info_voting = governance_contract.query_period_info(&mut app).unwrap();
    println!("\n\n period_info_voting{:#?}", period_info_voting);
    assert_eq!(
        period_info_voting,
        PeriodInfoResponse {
            current_block: 12349,
            current_period: ProposalPeriod::Voting,
            current_time_in_cycle: 30,
            current_posting_start: 1660000000,
            current_voting_start: 1660000020,
            current_voting_end: 1660000040,
            next_posting_start: 1660000040,
            next_voting_start: 1660000060,
            posting_period_length: 20,
            voting_period_length: 20,
            cycle_length: 40
        }
    );

    // User1 votes no on the governance proposal to fail it

    let fund_proposal_vote = governance_contract
        .vote(&mut app, &user1, 1, VoteOption::No)
        .unwrap();
    println!("\n\n fund_proposal_vote {:?}", fund_proposal_vote);

    let proposal_result = governance_contract.query_proposal(&mut app, 1).unwrap();
    println!("\n\n proposal_result {:?}", proposal_result);

    // Test that you can't conclude a proposal in the voting period
    let voting_not_ended_err = governance_contract
        .conclude(&mut app, &user1, 1)
        .unwrap_err();
    assert_eq!(voting_not_ended_err, ContractError::VotingPeriodNotEnded {});

    // Skip period from Voting to Posting so we can conclude the prBLOoKSECNDS
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.voting_period_length);
        block.height += period_info_posting.voting_period_length / SECONDS_PER_BLOCK;
    });

    let period_info_posting2 = governance_contract.query_period_info(&mut app).unwrap();
    println!("\n\n period_info_posting2 {:?}", period_info_posting2);

    let conclude_proposal_result = governance_contract.conclude(&mut app, &user1, 1).unwrap();
    println!(
        "\n\n conclude_proposal_result {:?}",
        conclude_proposal_result
    );

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = governance_contract
        .conclude(&mut app, &user1, 1)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );
    println!(
        "\n\n conclude2_proposal_result {:?}",
        conclude2_proposal_result
    );

    // Skip half the grant duration time to allow us to test if the failing proposal lets us claim funds
    app.update_block(|mut block| {
        block.time = block.time.plus_seconds(FUNDING_DURATION / 2);
        block.height += FUNDING_DURATION / 2 / SECONDS_PER_BLOCK;
    });

    let claim_funds_err = distribution_contract
        .claim(&mut app, &user1, 1)
        .unwrap_err();
    assert_eq!(
        claim_funds_err,
        distribution::ContractError::GrantNotFound {}
    );

    assert_eq!(
        app.wrap()
            .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
            .unwrap(),
        vec![]
    );
    assert_eq!(
        app.wrap()
            .query_all_balances(distribution_contract.addr().clone())
            .unwrap(),
        coins(DISTRIBUTION_INIT_BALANCE, "uluna")
    );
}

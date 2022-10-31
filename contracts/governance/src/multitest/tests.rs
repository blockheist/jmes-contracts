#![cfg(test)]

use cosmwasm_std::{
    coins,
    testing::{mock_env, MockApi, MockStorage},
    to_binary, Addr, Coin, CosmosMsg, Timestamp, Uint128, WasmMsg,
};
use cw_multi_test::{App, AppBuilder, BankKeeper};
use cw_utils::{Duration, Threshold};
use dao::multitest::contract::DaoContract;
use distribution::multitest::contract::DistributionContract;
use identityservice::multitest::contract::IdentityserviceContract;
use jmes::msg::Voter;

// use crate::error::ContractError;

use crate::{
    error::ContractError,
    msg::{Cw20HookMsg, PeriodInfoResponse, ProposalPeriod},
    state::VoteOption,
};

use super::contract::GovernanceContract;
use bjmes_token::multitest::contract::BjmesTokenContract;

const BLOCKS_PER_SECONDS: u64 = 5;
const PROPOSAL_REQUIRED_DEPOSIT: u128 = 1000;
const EPOCH_START: u64 = 1660000010;

const USER1_VOTING_COINS: u128 = 2000;
const USER2_VOTING_COINS: u128 = 3000;

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

#[test]
fn governance_funding_proposal_passing() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    const FUNDING_DURATION: u64 = 1000000u64;
    const FUNDING_AMOUNT: u128 = 1000000u128;

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
                amount: Uint128::from(1000000u128),
            }],
        )
    });

    println!("\n\n fund_res {:?}", fund_res);

    // Mint bjmes tokens to my_dao_addr so it can send the deposit
    let mint2_res = bjmes_contract
        .mint(
            &mut app,
            &user1,
            my_dao_addr.clone(),
            Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
        )
        .unwrap();

    println!("\n\nmint2_res {:?}", mint2_res);

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
    let proposal_msg = Cw20HookMsg::Funding {
        title: "Funding".to_string(),
        description: "Give me money".to_string(),
        duration: FUNDING_DURATION,
        amount: Uint128::from(FUNDING_AMOUNT),
    };

    // bondedJMES token send Msg (forwards the proposalMsg to the governance contract)
    let cw20_send_msg = bjmes_token::msg::ExecuteMsg::Send {
        contract: governance_contract.addr().clone().into(),
        amount: PROPOSAL_REQUIRED_DEPOSIT.into(),
        msg: to_binary(&proposal_msg).unwrap(),
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: bjmes_contract.addr().clone().into(),
        msg: to_binary(&cw20_send_msg).unwrap(),
        funds: vec![],
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

    // Skip period from Posting to Voting
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / BLOCKS_PER_SECONDS;
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

    // Skip period from Voting to Posting so we can conclude the proposal
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.voting_period_length);
        block.height += period_info_posting.voting_period_length / BLOCKS_PER_SECONDS;
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
        block.height += FUNDING_DURATION / 2 / BLOCKS_PER_SECONDS;
    });

    let claim_funds_result = distribution_contract.claim(&mut app, &user1, 1).unwrap();
    println!("\n\n claim_funds_result {:?}", claim_funds_result);

    assert_eq!(
        app.wrap()
            .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
            .unwrap(),
        coins(500000, "uluna")
    );

    // Skip double the grant duration time to claim 100% of the funds
    app.update_block(|mut block| {
        block.time = block.time.plus_seconds(FUNDING_DURATION * 2);
        block.height += FUNDING_DURATION * 2 / BLOCKS_PER_SECONDS;
    });

    let claim_funds_result = distribution_contract
        .claim(&mut app, &Addr::unchecked(my_dao_addr.clone()), 1)
        .unwrap();
    println!("\n\n claim_funds_result {:?}", claim_funds_result);

    assert_eq!(
        app.wrap()
            .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
            .unwrap(),
        coins(1000000, "uluna")
    );

    // Skip period from Posting to Voting
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / BLOCKS_PER_SECONDS;
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

    const FUNDING_DURATION: u64 = 1000000u64;
    const FUNDING_AMOUNT: u128 = 1000000u128;

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
                amount: Uint128::from(99999999999u128),
            }],
        )
    });

    println!("\n\n fund_res {:?}", fund_res);

    // Mint bjmes tokens to my_dao_addr so it can send the deposit
    let mint2_res = bjmes_contract
        .mint(
            &mut app,
            &user1,
            my_dao_addr.clone(),
            Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
        )
        .unwrap();

    println!("\n\nmint2_res {:?}", mint2_res);

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
    let proposal_msg = Cw20HookMsg::Funding {
        title: "Funding".to_string(),
        description: "Give me money".to_string(),
        duration: FUNDING_DURATION,
        amount: Uint128::from(FUNDING_AMOUNT),
    };

    // bondedJMES token send Msg (forwards the proposalMsg to the governance contract)
    let cw20_send_msg = bjmes_token::msg::ExecuteMsg::Send {
        contract: governance_contract.addr().clone().into(),
        amount: PROPOSAL_REQUIRED_DEPOSIT.into(),
        msg: to_binary(&proposal_msg).unwrap(),
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: bjmes_contract.addr().clone().into(),
        msg: to_binary(&cw20_send_msg).unwrap(),
        funds: vec![],
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

    // Skip period from Posting to Voting
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / BLOCKS_PER_SECONDS;
    });

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

    // Skip period from Voting to Posting so we can conclude the proposal
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.voting_period_length);
        block.height += period_info_posting.voting_period_length / BLOCKS_PER_SECONDS;
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
        block.height += FUNDING_DURATION / 2 / BLOCKS_PER_SECONDS;
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
}

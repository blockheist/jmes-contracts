#![cfg(test)]
use std::mem;

use cosmwasm_std::{
    coins, from_binary,
    testing::{mock_env, MockApi, MockStorage},
    to_binary, Addr, Coin, CosmosMsg, Timestamp, Uint128, WasmMsg,
};
use cw4::Member;
use cw_multi_test::{next_block, App, AppBuilder, AppResponse, BankKeeper, Wasm};
use cw_utils::Duration;
use dao_members::multitest::contract::DaoMembersContract;
use dao_multisig::{msg::ProposeResponse, multitest::contract::DaoMultisigContract};
use identityservice::multitest::contract::IdentityserviceContract;
use serde::__private::de::IdentifierDeserializer;

use crate::{
    error::ContractError,
    msg::{CoreSlot, ExecuteMsg, ProposalMsg, ProposalPeriod, ProposalResponse},
    state::{Funding, ProposalStatus, VoteOption},
};

use super::contract::GovernanceContract;

const SECONDS_PER_BLOCK: u64 = 5;
const PROPOSAL_REQUIRED_DEPOSIT: u128 = 10_000_000; // 10 JMES
const PROPOSAL_REQUIRED_PERCENTAGE: u64 = 10; // 10% NET YES votes required

const PERIOD_START_EPOCH: u64 = 0;
const POSTING_PERIOD_LENGTH: u64 = 40;
const VOTING_PERIOD_LENGTH: u64 = 40;

const USER1_FUNDING_COINS: u128 = 1000_000_000;
const USER1_VOTING_COINS: u128 = 2000_000_000;
const USER2_VOTING_COINS: u128 = 3000_000_000;

#[derive(Debug, Clone)]
struct Contracts {
    governance: GovernanceContract,
    identityservice: IdentityserviceContract,
}

// Helper functions to keep the tests more succinct
fn mock_app() -> App {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(PERIOD_START_EPOCH);
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

fn instantiate_contracts(app: &mut App, user1: Addr, user2: Addr, owner: Addr) -> Contracts {
    // Instantiate needed contracts

    let governance_code_id = GovernanceContract::store_code(app);
    println!("governance_code_id {:?}", governance_code_id);
    let governance_contract = GovernanceContract::instantiate(
        app,
        governance_code_id,
        &user1,
        "Governance Contract",
        owner.clone().into(),
        Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
        PROPOSAL_REQUIRED_PERCENTAGE,
        PERIOD_START_EPOCH,
        POSTING_PERIOD_LENGTH,
        VOTING_PERIOD_LENGTH,
    )
    .unwrap();

    let dao_members_code_id = DaoMembersContract::store_code(app);
    println!("dao_members_code_id: {}", dao_members_code_id);
    let dao_multisig_code_id = DaoMultisigContract::store_code(app);
    println!("dao_multisig_code_id {:?}", dao_multisig_code_id);

    let identityservice_code_id = IdentityserviceContract::store_code(app);
    let identityservice_contract = IdentityserviceContract::instantiate(
        app,
        identityservice_code_id,
        &user1,
        "identityservice",
        dao_members_code_id,
        dao_multisig_code_id,
        governance_contract.addr().clone(),
    )
    .unwrap();

    println!("\n\ngovernance_contract {:?}", governance_contract);
    println!("identityservice_contract {:?}", identityservice_contract);

    // Set contract options
    governance_contract
        .set_contract(
            app,
            &owner,
            "art_dealer".into(), // TODO instantiate art_dealer contract and use actual address
            identityservice_contract.addr().into(),
        )
        .unwrap();

    // Fund the governance contract
    app.init_modules(|router, _, storage| {
        // router
        //     .bank
        //     .init_balance(
        //         storage,
        //         governance_contract.addr(),
        //         vec![Coin {
        //             denom: "ujmes".to_string(),
        //             amount: Uint128::from(GOVERNANCE_INIT_BALANCE),
        //         }],
        //     )
        // .unwrap();

        // Mint tokens to user1
        router
            .bank
            .init_balance(
                storage,
                &user1.clone(),
                vec![
                    // Mint jmes tokens to user1 so it can pay the proposal fee
                    Coin {
                        denom: "ujmes".to_string(),
                        amount: Uint128::from(USER1_FUNDING_COINS),
                    },
                    // Mint bjmes tokens to user1 so it can vote
                    Coin {
                        denom: "bujmes".to_string(),
                        amount: Uint128::from(USER1_VOTING_COINS),
                    },
                ],
            )
            .unwrap();

        // Mint bjmes tokens to user2 so it can vote
        router
            .bank
            .init_balance(
                storage,
                &user2.clone(),
                vec![Coin {
                    denom: "bujmes".to_string(),
                    amount: Uint128::from(USER2_VOTING_COINS),
                }],
            )
            .unwrap();
    });

    // Produce a block to mine balances
    app.update_block(|mut block| {
        block.time = Timestamp::from_seconds(block.time.seconds() + SECONDS_PER_BLOCK);
        block.height += 1;
    });

    Contracts {
        governance: governance_contract,
        identityservice: identityservice_contract,
    }
}

fn create_dao(app: &mut App, contracts: Contracts, user1_addr: Addr, user2_addr: Addr) -> Addr {
    create_dao_from(
        app,
        contracts,
        vec![
            Member {
                addr: user1_addr.into(),
                weight: 26,
            },
            Member {
                addr: user2_addr.into(),
                weight: 26,
            },
        ],
    )
}

fn create_dao_from(app: &mut App, contracts: Contracts, members: Vec<Member>) -> Addr {
    // Register dao identity with valid name
    let my_dao = contracts
        .identityservice
        .register_dao(
            app,
            &Addr::unchecked(members[0].addr.clone()),
            members,
            "mydao".to_string(),
            51u64,
            Duration::Time(2000000),
        )
        .unwrap();

    let my_dao_addr = from_binary::<dao_multisig::msg::InstantiateResponse>(&my_dao.data.unwrap())
        .unwrap()
        .dao_multisig_addr;
    assert_eq!(my_dao_addr, "contract3");

    app.update_block(next_block);
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
) -> Result<AppResponse, ContractError> {
    let period_info_posting = contracts.governance.query_period_info(app).unwrap();
    println!("\n\n period_info_posting {:?}", period_info_posting);
    assert_eq!(period_info_posting.current_period, ProposalPeriod::Posting);

    // Skip period from Posting to Voting
    app.update_block(|mut block| {
        block.time = block
            .time
            .plus_seconds(period_info_posting.posting_period_length);
        block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
    });

    let period_info_voting = contracts.governance.query_period_info(app).unwrap();
    println!("\n\n period_info_voting{:?}", period_info_voting);
    // assert_eq!(
    //     period_info_voting,
    //     PeriodInfoResponse {
    //         current_block: 12350,
    //         current_period: ProposalPeriod::Voting,
    //         current_time_in_cycle: 35,
    //         current_posting_start: 1660000000,
    //         current_voting_start: 1660000020,
    //         current_voting_end: 1660000040,
    //         next_posting_start: 1660000040,
    //         next_voting_start: 1660000060,
    //         posting_period_length: 20,
    //         voting_period_length: 20,
    //         cycle_length: 40
    //     }
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

    let conclude_proposal_result = contracts.governance.conclude(app, &user1, proposal_id);

    println!(
        "\n\n conclude_proposal_result {:?}",
        conclude_proposal_result
    );

    conclude_proposal_result
}

// The actual tests
#[test]
fn text_proposal_no_funding_attached() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    println!("my_dao_addr {:#?}", my_dao_addr);

    // Create a DAO proposal for a Gov Text Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::TextProposal {
        title: "First Text Proposal".into(),
        description: "Text Proposal Description".into(),
        funding: None,
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg).unwrap(),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    let proposal_id = 1;

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        user1.clone(),
        VoteOption::Yes,
        user2.clone(),
        VoteOption::No,
        proposal_id,
    )
    .unwrap();

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = contracts
        .governance
        .conclude(&mut app, &user1, proposal_id)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );

    let final_proposal = contracts
        .governance
        .query_proposal(&mut app, proposal_id)
        .unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);
}

#[test]
fn text_proposal_with_funding_attached_amount_larger_0() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    println!("my_dao_addr {:#?}", my_dao_addr);

    // Create a DAO proposal for a Gov Text Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::TextProposal {
        title: "First Text Proposal".into(),
        description: "Text Proposal Description".into(),
        funding: Some(Funding {
            amount: Uint128::from(100_000_000u128),
            duration_in_blocks: 100,
        }),
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg).unwrap(),
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
    )
    .unwrap();

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = contracts
        .governance
        .conclude(&mut app, &user1, 1)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);
}
#[test]
fn text_proposal_with_funding_attached_amount_equal_0() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    println!("my_dao_addr {:#?}", my_dao_addr);

    // Create a DAO proposal for a Gov Text Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::TextProposal {
        title: "First Text Proposal".into(),
        description: "Text Proposal Description".into(),
        funding: Some(Funding {
            amount: Uint128::zero(),
            duration_in_blocks: 100,
        }),
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        user1.clone(),
        user2.clone(),
        to_binary(&proposal_msg).unwrap(),
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
    )
    .unwrap();

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = contracts
        .governance
        .conclude(&mut app, &user1, 1)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);
}
#[test]
fn dao_membership_update_proposal_succeeds() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");
    let user3 = Addr::unchecked("user3");

    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    // Register the DAO
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    println!("my_dao_addr {:#?}", my_dao_addr);

    // Create the proposal to update the membership
    let proposal_msg = dao_members::msg::ExecuteMsg::UpdateMembers {
        remove: vec![user1.clone().into()],
        add: vec![
            Member {
                addr: user2.clone().into(),
                weight: 55,
            },
            Member {
                addr: user3.clone().into(),
                weight: 5,
            },
        ],
    };

    println!("\n\n proposal_msg {:?}", proposal_msg);

    let dao_multisig_config =
        DaoMultisigContract::query_config(&mut app, my_dao_addr.clone()).unwrap();

    // Wrap  proposal msg so we can attach it to the dao proposal
    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_multisig_config.dao_members_addr.clone().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        funds: vec![],
    };

    let dao_propose_response = DaoMultisigContract::propose(
        &mut app,
        &user1,
        &my_dao_addr.clone().into(),
        "Dao Proposal".into(),
        "Attaches Membership update msg".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
        PROPOSAL_REQUIRED_DEPOSIT,
    );

    let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response.unwrap().data.unwrap())
        .unwrap()
        .proposal_id;

    println!("\n\n proposal_id {:?}", proposal_id);

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoMultisigContract::vote(
        &mut app,
        &user2,
        &my_dao_addr.clone().into(),
        proposal_id,
        cw3::Vote::Yes,
    );
    println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

    let dao_execute_result =
        DaoMultisigContract::execute(&mut app, &user1, &my_dao_addr.clone().into(), proposal_id);
    println!("\n\n dao_execute_result {:?}", dao_execute_result);

    let multisig_voter_list =
        DaoMultisigContract::query_list_voters(&app, my_dao_addr.clone()).unwrap();
    println!("\n\n voter_list {:?}", multisig_voter_list);

    let member_list =
        DaoMembersContract::query_list_members(&app, dao_multisig_config.dao_members_addr).unwrap();
    println!("\n\n member_list {:?}", member_list);

    // Check that the multisig voter list is the same as the dao member list
    assert_eq!(
        multisig_voter_list.voters[0].addr,
        member_list.members[0].addr
    );
    assert_eq!(
        multisig_voter_list.voters[0].weight,
        member_list.members[0].weight
    );
    assert_eq!(
        multisig_voter_list.voters[1].addr,
        member_list.members[1].addr
    );
    assert_eq!(
        multisig_voter_list.voters[1].weight,
        member_list.members[1].weight
    );
}
#[test]
fn core_dao_membership_update_2_members_fails() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let member1 = Member {
        addr: "member1".into(),
        weight: 26,
    };
    let member2 = Member {
        addr: "member2".into(),
        weight: 26,
    };
    let member3 = Member {
        addr: "member3".into(),
        weight: 10,
    };

    let members = vec![member1.clone(), member2.clone(), member3.clone()];

    let contracts = instantiate_contracts(
        &mut app,
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        owner.clone(),
    );

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(
            &mut app,
            &Addr::unchecked(member1.addr.clone()),
            "user1id".to_string(),
        )
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao_from(&mut app, contracts.clone(), members);

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        funding: Funding {
            amount: 10_000_000u128.into(),
            duration_in_blocks: 3000,
        },
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        to_binary(&proposal_msg).unwrap(),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        Addr::unchecked(member1.addr.clone()),
        VoteOption::Yes,
        Addr::unchecked(member2.addr.clone()),
        VoteOption::No,
        1,
    )
    .unwrap();

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = contracts
        .governance
        .conclude(&mut app, &Addr::unchecked(member1.clone().addr), 1)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);
    assert_eq!(
        final_proposal,
        ProposalResponse {
            id: 1,
            dao: Addr::unchecked("contract3"),
            title: "Make me CoreTech".into(),
            description: "Serving the chain".into(),
            prop_type: crate::state::ProposalType::CoreSlot(CoreSlot::Brand {}),
            coins_yes: Uint128::from(2000000000u128),
            coins_no: Uint128::from(0u128),
            coins_total: Uint128::from(5000000000u128),
            yes_voters: vec![Addr::unchecked("member1")],
            no_voters: vec![],
            deposit_amount: Uint128::from(10000000u128),
            start_block: 12347,
            posting_start: 0,
            voting_start: 40,
            voting_end: 80,
            concluded_at_height: Some(12363),
            status: ProposalStatus::SuccessConcluded,
            funding: Some(Funding {
                amount: Uint128::from(10000000u128),
                duration_in_blocks: 3000
            }),
            msgs: Some(vec![crate::multitest::tests::CosmosMsg::Wasm(
                crate::multitest::tests::WasmMsg::Execute {
                    contract_addr: "contract0".into(),
                    msg: to_binary(&crate::ExecuteMsg::SetCoreSlot { proposal_id: 1 }).unwrap(),
                    funds: vec![]
                }
            )]),
        }
    );

    // Create the proposal to update the membership
    let proposal_msg = dao_members::msg::ExecuteMsg::UpdateMembers {
        remove: vec![member1.clone().addr],
        add: vec![],
    };

    println!("\n\n proposal_msg {:?}", proposal_msg);

    let dao_multisig_config =
        DaoMultisigContract::query_config(&mut app, my_dao_addr.clone()).unwrap();

    // Wrap  proposal msg so we can attach it to the dao proposal
    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_multisig_config.dao_members_addr.clone().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        funds: vec![],
    };

    let dao_propose_response = DaoMultisigContract::propose(
        &mut app,
        &Addr::unchecked(member1.addr.clone()),
        &my_dao_addr.clone().into(),
        "Dao Proposal".into(),
        "Wraps Governance Proposal".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
        PROPOSAL_REQUIRED_DEPOSIT,
    );

    let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response.unwrap().data.unwrap())
        .unwrap()
        .proposal_id;

    println!("\n\n proposal_id {:?}", proposal_id);

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoMultisigContract::vote(
        &mut app,
        &Addr::unchecked(member2.addr),
        &my_dao_addr.clone().into(),
        proposal_id,
        cw3::Vote::Yes,
    );
    println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

    let dao_execute_result = DaoMultisigContract::execute(
        &mut app,
        &Addr::unchecked(member1.addr),
        &my_dao_addr.clone().into(),
        proposal_id,
    );
    println!("\n\n dao_execute_result {:?}", dao_execute_result);

    assert_eq!(
        dao_execute_result.unwrap_err(),
        dao_multisig::ContractError::DowncastError {
            text: "WrongCoreTeamMemberCount (Core Team must have between 3 and 9 members)!"
                .to_string()
        }
    );
}
#[test]
fn core_dao_membership_update_1_exceeds_threshold_fails() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let member1 = Member {
        addr: "member1".into(),
        weight: 26,
    };
    let member2 = Member {
        addr: "member2".into(),
        weight: 26,
    };
    let member3 = Member {
        addr: "member3".into(),
        weight: 10,
    };

    let members = vec![member1.clone(), member2.clone(), member3.clone()];

    let contracts = instantiate_contracts(
        &mut app,
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        owner.clone(),
    );

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(
            &mut app,
            &Addr::unchecked(member1.addr.clone()),
            "user1id".to_string(),
        )
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao_from(&mut app, contracts.clone(), members);

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        funding: Funding {
            amount: 10_000_000u128.into(),
            duration_in_blocks: 3000,
        },
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        to_binary(&proposal_msg).unwrap(),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        Addr::unchecked(member1.addr.clone()),
        VoteOption::Yes,
        Addr::unchecked(member2.addr.clone()),
        VoteOption::No,
        1,
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);
    assert_eq!(
        final_proposal,
        ProposalResponse {
            id: 1,
            dao: Addr::unchecked("contract3"),
            title: "Make me CoreTech".into(),
            description: "Serving the chain".into(),
            prop_type: crate::state::ProposalType::CoreSlot(CoreSlot::Brand {}),
            coins_yes: Uint128::from(2000000000u128),
            coins_no: Uint128::from(0u128),
            coins_total: Uint128::from(5000000000u128),
            yes_voters: vec![Addr::unchecked("member1")],
            no_voters: vec![],
            deposit_amount: Uint128::from(10000000u128),
            start_block: 12347,
            posting_start: 0,
            voting_start: 40,
            voting_end: 80,
            concluded_at_height: Some(12363),
            status: ProposalStatus::SuccessConcluded,
            funding: Some(Funding {
                amount: Uint128::from(10000000u128),
                duration_in_blocks: 3000
            }),
            msgs: Some(vec![crate::multitest::tests::CosmosMsg::Wasm(
                crate::multitest::tests::WasmMsg::Execute {
                    contract_addr: "contract0".into(),
                    msg: to_binary(&crate::ExecuteMsg::SetCoreSlot { proposal_id: 1 }).unwrap(),
                    funds: vec![]
                }
            )]),
        }
    );

    // Create the proposal to update the membership
    let proposal_msg = dao_members::msg::ExecuteMsg::UpdateMembers {
        remove: vec![member1.clone().addr],
        add: vec![Member {
            addr: "member4".into(),
            weight: 51, // Adding voting power >= theshold which should fail in a core dao update membership proposal
        }],
    };

    println!("\n\n proposal_msg {:?}", proposal_msg);

    let dao_multisig_config =
        DaoMultisigContract::query_config(&mut app, my_dao_addr.clone()).unwrap();

    // Wrap  proposal msg so we can attach it to the dao proposal
    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_multisig_config.dao_members_addr.clone().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        funds: vec![],
    };

    let dao_propose_response = DaoMultisigContract::propose(
        &mut app,
        &Addr::unchecked(member1.addr.clone()),
        &my_dao_addr.clone().into(),
        "Dao Proposal".into(),
        "Wraps Governance Proposal".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
        PROPOSAL_REQUIRED_DEPOSIT,
    );

    let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response.unwrap().data.unwrap())
        .unwrap()
        .proposal_id;

    println!("\n\n proposal_id {:?}", proposal_id);

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoMultisigContract::vote(
        &mut app,
        &Addr::unchecked(member2.addr),
        &my_dao_addr.clone().into(),
        proposal_id,
        cw3::Vote::Yes,
    );
    println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

    let dao_execute_result = DaoMultisigContract::execute(
        &mut app,
        &Addr::unchecked(member1.addr),
        &my_dao_addr.clone().into(),
        proposal_id,
    );
    println!("\n\n dao_execute_result {:?}", dao_execute_result);

    assert_eq!(
        dao_execute_result.unwrap_err(),
        dao_multisig::ContractError::DowncastError {
            text: "WrongCoreTeamMemberVotingPower (Each Core Team must have less than AbsoluteCount { weight: 51 } but one member has 51 voting power)!".to_string()
        }
    );
}

#[test]
fn propose_core_slot_brand_with_2_members_fails() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");
    //
    let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(&mut app, &user1, "user1id".to_string())
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        funding: Funding {
            amount: 10_000_000u128.into(),
            duration_in_blocks: 3000,
        },
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    let my_dao_addr = my_dao_addr.to_string();

    // Wrap gov proposal msg so we can attach it to the dao proposal
    let wasm_msg = WasmMsg::Execute {
        contract_addr: contracts.governance.addr().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        // attach the proposal fee to be burned,
        // this is sent from the dao addr to the gov contract
        funds: coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
    };

    let dao_propose_response = DaoMultisigContract::propose(
        &mut app,
        &user1,
        &my_dao_addr,
        "Dao Proposal".into(),
        "Wraps Governance Proposal".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
        PROPOSAL_REQUIRED_DEPOSIT,
    );

    let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response.unwrap().data.unwrap())
        .unwrap()
        .proposal_id;

    println!("\n\n proposal_id {:?}", proposal_id);

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result =
        DaoMultisigContract::vote(&mut app, &user2, &my_dao_addr, proposal_id, cw3::Vote::Yes);
    println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

    let dao_execute_result =
        DaoMultisigContract::execute(&mut app, &user1, &my_dao_addr, proposal_id);
    println!("\n\n dao_execute_result {:?}", dao_execute_result);
    assert_eq!(
        dao_execute_result.unwrap_err(),
        dao_multisig::ContractError::DowncastError {
            text: "WrongCoreTeamMemberCount (Core Team must have between 3 and 9 members)!"
                .to_string()
        }
    );
}
#[test]
fn propose_core_slot_brand_with_3_members_2_required_for_threshold_succeeds() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let member1 = Member {
        addr: "member1".into(),
        weight: 26,
    };
    let member2 = Member {
        addr: "member2".into(),
        weight: 26,
    };
    let member3 = Member {
        addr: "member3".into(),
        weight: 10,
    };

    let members = vec![member1.clone(), member2.clone(), member3.clone()];

    let contracts = instantiate_contracts(
        &mut app,
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        owner.clone(),
    );

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(
            &mut app,
            &Addr::unchecked(member1.addr.clone()),
            "user1id".to_string(),
        )
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao_from(&mut app, contracts.clone(), members);

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        funding: Funding {
            amount: 10_000_000u128.into(),
            duration_in_blocks: 3000,
        },
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        to_binary(&proposal_msg).unwrap(),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Vote on and execute the governance proposal
    gov_vote_helper(
        &mut app,
        contracts.clone(),
        Addr::unchecked(member1.addr.clone()),
        VoteOption::Yes,
        Addr::unchecked(member2.addr.clone()),
        VoteOption::No,
        1,
    )
    .unwrap();

    // Test that you can't conclude a proposal (and execute its msgs) a second time
    let conclude2_proposal_result = contracts
        .governance
        .conclude(&mut app, &Addr::unchecked(member1.clone().addr), 1)
        .unwrap_err();
    assert_eq!(
        conclude2_proposal_result,
        ContractError::ProposalAlreadyConcluded {}
    );

    let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
    println!("\n\n final_proposal {:?}", final_proposal);
    assert_eq!(
        final_proposal,
        ProposalResponse {
            id: 1,
            dao: Addr::unchecked("contract3"),
            title: "Make me CoreTech".into(),
            description: "Serving the chain".into(),
            prop_type: crate::state::ProposalType::CoreSlot(CoreSlot::Brand {}),
            coins_yes: Uint128::from(2000000000u128),
            coins_no: Uint128::from(0u128),
            coins_total: Uint128::from(5000000000u128),
            yes_voters: vec![Addr::unchecked("member1")],
            no_voters: vec![],
            deposit_amount: Uint128::from(10000000u128),
            start_block: 12347,
            posting_start: 0,
            voting_start: 40,
            voting_end: 80,
            concluded_at_height: Some(12363),
            status: ProposalStatus::SuccessConcluded,
            funding: Some(Funding {
                amount: Uint128::from(10000000u128),
                duration_in_blocks: 3000
            }),
            msgs: Some(vec![crate::multitest::tests::CosmosMsg::Wasm(
                crate::multitest::tests::WasmMsg::Execute {
                    contract_addr: "contract0".into(),
                    msg: to_binary(&crate::ExecuteMsg::SetCoreSlot { proposal_id: 1 }).unwrap(),
                    funds: vec![]
                }
            )]),
        }
    )
}
#[test]
fn propose_core_slot_brand_with_3_members_1_exceeds_threshold_fails() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let member1 = Member {
        addr: "member1".into(),
        weight: 20,
    };
    let member2 = Member {
        addr: "member2".into(),
        weight: 20,
    };
    let member3 = Member {
        addr: "member3".into(),
        weight: 60,
    };

    let members = vec![member1.clone(), member2.clone(), member3.clone()];

    let contracts = instantiate_contracts(
        &mut app,
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        owner.clone(),
    );

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(
            &mut app,
            &Addr::unchecked(member1.addr.clone()),
            "user1id".to_string(),
        )
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao_from(&mut app, contracts.clone(), members);

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        funding: Funding {
            amount: 10_000_000u128.into(),
            duration_in_blocks: 3000,
        },
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    let my_dao_addr = my_dao_addr.to_string();

    // Wrap gov proposal msg so we can attach it to the dao proposal
    let wasm_msg = WasmMsg::Execute {
        contract_addr: contracts.governance.addr().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        // attach the proposal fee to be burned,
        // this is sent from the dao addr to the gov contract
        funds: coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
    };

    let dao_propose_response = DaoMultisigContract::propose(
        &mut app,
        &Addr::unchecked(member1.addr.clone()),
        &my_dao_addr,
        "Dao Proposal".into(),
        "Wraps Governance Proposal".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
        PROPOSAL_REQUIRED_DEPOSIT,
    );

    let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response.unwrap().data.unwrap())
        .unwrap()
        .proposal_id;

    println!("\n\n proposal_id {:?}", proposal_id);

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoMultisigContract::vote(
        &mut app,
        &Addr::unchecked(member3.addr.clone()),
        &my_dao_addr,
        proposal_id,
        cw3::Vote::Yes,
    );
    println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

    let dao_execute_result = DaoMultisigContract::execute(
        &mut app,
        &Addr::unchecked(member1.addr.clone()),
        &my_dao_addr,
        proposal_id,
    );
    println!("\n\n dao_execute_result {:?}", dao_execute_result);
    assert_eq!(
        dao_execute_result.unwrap_err(),
        dao_multisig::ContractError::DowncastError {
            text: "WrongCoreTeamMemberVotingPower (Each Core Team must have less than AbsoluteCount { weight: 51 } but one members has 60 voting power)!".to_string()
        }
    );
}
#[test]
fn propose_core_slot_update_membership_before_conclude_1_exceeds_threshold_set_core_slot_fails() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let member1 = Member {
        addr: "member1".into(),
        weight: 26,
    };
    let member2 = Member {
        addr: "member2".into(),
        weight: 26,
    };
    let member3 = Member {
        addr: "member3".into(),
        weight: 10,
    };

    let members = vec![member1.clone(), member2.clone(), member3.clone()];

    let contracts = instantiate_contracts(
        &mut app,
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        owner.clone(),
    );

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(
            &mut app,
            &Addr::unchecked(member1.addr.clone()),
            "user1id".to_string(),
        )
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao_from(&mut app, contracts.clone(), members);

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        funding: Funding {
            amount: 10_000_000u128.into(),
            duration_in_blocks: 3000,
        },
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        to_binary(&proposal_msg).unwrap(),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Create the proposal to update the membership
    let proposal_msg = dao_members::msg::ExecuteMsg::UpdateMembers {
        remove: vec![],
        add: vec![Member {
            addr: "member1".into(),
            weight: 55,
        }],
    };

    println!("\n\n proposal_msg {:?}", proposal_msg);

    let dao_multisig_config =
        DaoMultisigContract::query_config(&mut app, my_dao_addr.clone()).unwrap();

    // Wrap  proposal msg so we can attach it to the dao proposal
    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_multisig_config.dao_members_addr.clone().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        funds: vec![],
    };

    let dao_propose_response = DaoMultisigContract::propose(
        &mut app,
        &Addr::unchecked(member1.addr.clone()),
        &my_dao_addr.clone().into(),
        "Dao Proposal".into(),
        "Wraps Governance Proposal".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
        PROPOSAL_REQUIRED_DEPOSIT,
    );

    let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response.unwrap().data.unwrap())
        .unwrap()
        .proposal_id;

    println!("\n\n proposal_id {:?}", proposal_id);

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoMultisigContract::vote(
        &mut app,
        &Addr::unchecked(member2.addr.clone()),
        &my_dao_addr.clone().into(),
        proposal_id,
        cw3::Vote::Yes,
    );
    println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

    let dao_execute_result = DaoMultisigContract::execute(
        &mut app,
        &Addr::unchecked(member1.clone().addr),
        &my_dao_addr.clone().into(),
        proposal_id,
    );
    println!("\n\n dao_execute_result {:?}", dao_execute_result);

    let multisig_voter_list =
        DaoMultisigContract::query_list_voters(&app, my_dao_addr.clone()).unwrap();
    println!("\n\n voter_list {:?}", multisig_voter_list);

    let member_list =
        DaoMembersContract::query_list_members(&app, dao_multisig_config.dao_members_addr).unwrap();
    println!("\n\n member_list {:?}", member_list);

    // Vote on and execute the governance proposal
    let result = gov_vote_helper(
        &mut app,
        contracts.clone(),
        Addr::unchecked(member1.addr.clone()),
        VoteOption::Yes,
        Addr::unchecked(member2.addr.clone()),
        VoteOption::No,
        1,
    )
    .unwrap_err();

    assert_eq!(
        result,
        ContractError::WrongCoreTeamMemberVotingPower {
            threshold: cw_utils::Threshold::AbsoluteCount { weight: 51 },
            current: 55
        }
    );
}
#[test]
fn propose_core_slot_update_membership_before_conclude_2_members_set_core_slot_fails() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let member1 = Member {
        addr: "member1".into(),
        weight: 26,
    };
    let member2 = Member {
        addr: "member2".into(),
        weight: 26,
    };
    let member3 = Member {
        addr: "member3".into(),
        weight: 10,
    };

    let members = vec![member1.clone(), member2.clone(), member3.clone()];

    let contracts = instantiate_contracts(
        &mut app,
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        owner.clone(),
    );

    println!("\n\n contracts {:#?}", contracts);

    // Register an user identity with a valid name
    contracts
        .identityservice
        .register_user(
            &mut app,
            &Addr::unchecked(member1.addr.clone()),
            "user1id".to_string(),
        )
        .unwrap();

    // Register a DAO (required for submitting a proposal)
    let my_dao_addr = create_dao_from(&mut app, contracts.clone(), members);

    // Create a Dao Proposal for a Governance CoreSlot Proposal
    let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
        title: "Make me CoreTech".into(),
        description: "Serving the chain".into(),
        funding: Funding {
            amount: 10_000_000u128.into(),
            duration_in_blocks: 3000,
        },
        slot: CoreSlot::Brand {},
    });

    // Create, vote on and execute the dao proposal
    DaoMultisigContract::gov_proposal_helper(
        &mut app,
        my_dao_addr.clone(),
        &contracts.governance.addr().clone(),
        Addr::unchecked(member1.addr.clone()),
        Addr::unchecked(member2.addr.clone()),
        to_binary(&proposal_msg).unwrap(),
        PROPOSAL_REQUIRED_DEPOSIT,
    )
    .unwrap();

    // Create the proposal to update the membership
    let proposal_msg = dao_members::msg::ExecuteMsg::UpdateMembers {
        remove: vec![member3.clone().addr],
        add: vec![],
    };

    println!("\n\n proposal_msg {:?}", proposal_msg);

    let dao_multisig_config =
        DaoMultisigContract::query_config(&mut app, my_dao_addr.clone()).unwrap();

    // Wrap  proposal msg so we can attach it to the dao proposal
    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_multisig_config.dao_members_addr.clone().into(),
        msg: to_binary(&proposal_msg).unwrap(),
        funds: vec![],
    };

    let dao_propose_response = DaoMultisigContract::propose(
        &mut app,
        &Addr::unchecked(member1.addr.clone()),
        &my_dao_addr.clone().into(),
        "Dao Proposal".into(),
        "Wraps Governance Proposal".into(),
        vec![CosmosMsg::Wasm(wasm_msg)],
        None,
        PROPOSAL_REQUIRED_DEPOSIT,
    );

    let proposal_id = from_binary::<ProposeResponse>(&dao_propose_response.unwrap().data.unwrap())
        .unwrap()
        .proposal_id;

    println!("\n\n proposal_id {:?}", proposal_id);

    // User1 already voted automatically
    // User2 votes yes to pass the proposal
    let dao_vote2_result = DaoMultisigContract::vote(
        &mut app,
        &Addr::unchecked(member2.addr.clone()),
        &my_dao_addr.clone().into(),
        proposal_id,
        cw3::Vote::Yes,
    );
    println!("\n\n dao_vote2_result ....{:?}", dao_vote2_result);

    let dao_execute_result = DaoMultisigContract::execute(
        &mut app,
        &Addr::unchecked(member1.clone().addr),
        &my_dao_addr.clone().into(),
        proposal_id,
    );
    println!("\n\n dao_execute_result {:?}", dao_execute_result);

    let multisig_voter_list =
        DaoMultisigContract::query_list_voters(&app, my_dao_addr.clone()).unwrap();
    println!("\n\n voter_list {:?}", multisig_voter_list);

    let member_list =
        DaoMembersContract::query_list_members(&app, dao_multisig_config.dao_members_addr).unwrap();
    println!("\n\n member_list {:?}", member_list);

    // Vote on and execute the governance proposal
    let result = gov_vote_helper(
        &mut app,
        contracts.clone(),
        Addr::unchecked(member1.addr.clone()),
        VoteOption::Yes,
        Addr::unchecked(member2.addr.clone()),
        VoteOption::No,
        1,
    )
    .unwrap_err();

    assert_eq!(
        result,
        ContractError::WrongCoreTeamMemberCount { min: 3, max: 9 }
    );
}
// #[test]
// fn set_core_slot_brand_then_revoke_fail_then_revoke_success() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");
//     //
//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

//     println!("\n\n contracts {:#?}", contracts);

//     // Register an user identity with a valid name
//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();

//     // Register a DAO (required for submitting a proposal)
//     let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

//     // Create a Dao Proposal for a Governance CoreSlot Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
//         title: "Make me CoreTech".into(),
//         description: "Serving the chain".into(),
//         funding: Funding {
//             amount: 10_000_000u128.into(),
//             duration_in_blocks: 3000,
//         },
//         slot: CoreSlot::Brand {},
//     });

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

// // Vote on and execute the governance proposal
// gov_vote_helper(
//     &mut app,
//     contracts.clone(),
//     user1.clone(),
//     VoteOption::Yes,
//     user2.clone(),
//     VoteOption::No,
//     1,
// );

// let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
// println!("\n\n final_proposal {:?}", final_proposal);

// // Check that my_dao_addr now has the CoreTech slot
// let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
// assert_eq!(core_slots.brand.unwrap().dao, my_dao_addr.clone());
// assert_eq!(
//     app.wrap().query_all_balances(BURN_ADDRESS).unwrap(),
//     coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes")
// );

// // Create a dao proposal to revoke from the DAO from the Brand slot
// let proposal_msg = ExecuteMsg::Propose(ProposalMsg::RevokeProposal {
//     title: "Remove Brand Dao".into(),
//     description: "Leave it vacant".into(),
//     revoke_proposal_id: 1,
// });

// // Failing Revoke Proposal

// // Fund dao so we can send the send proposal deposit
// app.send_tokens(
//     contracts.governance.addr().clone(),
//     Addr::unchecked(my_dao_addr.clone()),
//     &coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
// )
// .unwrap();

// // Create, vote on and execute the dao proposal
// DaoMultisigContract::gov_proposal_helper(
//     &mut app,
//     my_dao_addr.clone(),
//     &contracts.governance.addr().clone(),
//     user1.clone(),
//     user2.clone(),
//     to_binary(&proposal_msg).unwrap(),
//     PROPOSAL_REQUIRED_DEPOSIT,
// )
// .unwrap();

// // Vote on and execute the governance proposal
// let revoke_result = gov_vote_helper(
//     &mut app,
//     contracts.clone(),
//     user1.clone(),
//     VoteOption::No,
//     user2.clone(),
//     VoteOption::No,
//     2,
// );

// println!("\n\n revoke_result {:?}", revoke_result);

// let failing_proposal = contracts.governance.query_proposal(&mut app, 2).unwrap();
// assert_eq!(
//     failing_proposal,
//     ProposalResponse {
//         id: 2,
//         dao: my_dao_addr.clone(),
//         title: "Remove Brand Dao".into(),
//         description: "Leave it vacant".into(),
//         prop_type: crate::state::ProposalType::RevokeProposal(1u64),
//         coins_yes: Uint128::from(0u128),
//         coins_no: Uint128::from(2000_000_000u128),
//         yes_voters: vec![],
//         no_voters: vec![user1.clone()],
//         deposit_amount: Uint128::from(10_000u128),
//         start_block: 12363,
//         posting_start: 1660000080,
//         voting_start: 1660000120,
//         voting_end: 1660000160,
//         concluded_at_height: Some(3000u64),
//         status: ProposalStatus::ExpiredConcluded
//     }
// );

// println!("\n\n failing_proposal {:?}", failing_proposal);

// let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
// println!("\n\n core_slots {:?}", core_slots);
// assert_eq!(
//     core_slots.brand,
//     Some(SlotVoteResult {
//         dao: my_dao_addr.clone(),
//         yes_ratio: Decimal::percent(100),
//         proposal_voting_end: 1660000080,
//         proposal_id: 1u64,
//         proposal_funding_end: 30000u64
//     })
// );

// // Successful Revoke Proposal

// // Fund my_dao_addr so it can send the deposit
// app.send_tokens(
//     contracts.governance.addr().clone(),
//     Addr::unchecked(my_dao_addr.clone()),
//     &coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
// )
// .unwrap();

// // Create, vote on and execute the dao proposal
// DaoMultisigContract::gov_proposal_helper(
//     &mut app,
//     my_dao_addr.clone(),
//     &contracts.governance.addr().clone(),
//     user1.clone(),
//     user2.clone(),
//     to_binary(&proposal_msg).unwrap(),
//     PROPOSAL_REQUIRED_DEPOSIT,
// )
// .unwrap();

// // Vote on and execute the governance proposal
// let revoke_result = gov_vote_helper(
//     &mut app,
//     contracts.clone(),
//     user1.clone(),
//     VoteOption::Yes,
//     user2.clone(),
//     VoteOption::No,
//     3,
// );

// println!("\n\n revoke_result {:?}", revoke_result);

// let success_proposal = contracts.governance.query_proposal(&mut app, 3).unwrap();
// assert_eq!(
//     success_proposal,
//     ProposalResponse {
//         id: 3,
//         dao: my_dao_addr.clone(),
//         title: "Remove Brand Dao".into(),
//         description: "Leave it vacant".into(),
//         prop_type: crate::state::ProposalType::RevokeProposal(2u64),
//         coins_yes: Uint128::from(2000_000_000u128),
//         coins_no: Uint128::from(0u128),
//         yes_voters: vec![user1.clone()],
//         no_voters: vec![],
//         deposit_amount: Uint128::from(10_000u128),
//         start_block: 12379,
//         posting_start: 1660000160,
//         voting_start: 1660000200,
//         voting_end: 1660000240,
//         concluded_at_height: Some(3000u64),
//         status: ProposalStatus::SuccessConcluded
//     }
// );

// println!("\n\n success_proposal {:?}", success_proposal);

// let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
// println!("\n\n core_slots {:?}", core_slots);
// assert_eq!(core_slots.brand, None);
// }

// #[test]
// fn set_core_slot_creative_and_fail_setting_a_second_slot_for_the_same_dao() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");

//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

//     println!("\n\n contracts {:#?}", contracts);

//     // Register an user identity with a valid name
//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();

//     // Register a DAO (required for submitting a proposal)
//     let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

//     // Create a Dao Proposal for a Governance CoreSlot Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
//         title: "Make me CoreTech".into(),
//         description: "Serving the chain".into(),
//         funding: Funding {
//             amount: 10_000_000u128.into(),
//             duration_in_blocks: 3000,
//         },
//         slot: CoreSlot::Creative {},
//     });

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     // Vote on and execute the governance proposal
//     gov_vote_helper(
//         &mut app,
//         contracts.clone(),
//         user1.clone(),
//         VoteOption::Yes,
//         user2.clone(),
//         VoteOption::No,
//         1,
//     );

//     let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
//     println!("\n\n final_proposal {:?}", final_proposal);

//     // Check that my_dao_addr now has the Creative slot
//     let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
//     assert_eq!(core_slots.creative.unwrap().dao, my_dao_addr);

//     // Fail to set a second slot for the same dao

//     // Create a Dao Proposal for a Governance CoreSlot Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
//         title: "Make me CoreTech".into(),
//         description: "Serving the chain".into(),
//         funding: Funding {
//             amount: 10_000_000u128.into(),
//             duration_in_blocks: 3000,
//         },
//         slot: CoreSlot::Brand {},
//     });

//     // Fund dao so we can send the send proposal deposit
//     app.send_tokens(
//         contracts.governance.addr().clone(),
//         Addr::unchecked(my_dao_addr.clone()),
//         &coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
//     )
//     .unwrap();
//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     // Vote on and execute the governance proposal
//     let failed_core_slot_res = gov_vote_helper(
//         &mut app,
//         contracts.clone(),
//         user1.clone(),
//         VoteOption::Yes,
//         user2.clone(),
//         VoteOption::No,
//         2,
//     );

//     assert_eq!(
//         get_attribute(&failed_core_slot_res, "wasm", "error"),
//         "dao already holds a core slot".to_string()
//     );

//     let failed_proposal = contracts.governance.query_proposal(&mut app, 2).unwrap();
//     assert_eq!(failed_proposal.status, ProposalStatus::SuccessConcluded);
// }

// #[test]
// fn set_core_slot_tech_and_resign() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");

//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

//     println!("\n\n contracts {:#?}", contracts);

//     // Register an user identity with a valid name
//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();

//     // Register a DAO (required for submitting a proposal)
//     let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

//     // Create a Dao Proposal for a Governance CoreSlot Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
//         title: "Make me CoreTech".into(),
//         description: "Serving the chain".into(),
//         funding: Funding {
//             amount: 10_000_000u128.into(),
//             duration_in_blocks: 3000,
//         },
//         slot: CoreSlot::CoreTech {},
//     });

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     // Vote on and execute the governance proposal
//     gov_vote_helper(
//         &mut app,
//         contracts.clone(),
//         user1.clone(),
//         VoteOption::Yes,
//         user2.clone(),
//         VoteOption::No,
//         1,
//     );

//     let final_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
//     println!("\n\n final_proposal {:?}", final_proposal);

//     // Check that my_dao_addr now has the CoreTech slot
//     let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
//     assert_eq!(core_slots.core_tech.unwrap().dao, my_dao_addr);

//     // Create a dao proposal to resign from the CoreTech slot
//     let proposal_msg = ExecuteMsg::ResignCoreSlot {
//         slot: CoreSlot::CoreTech {},
//         note: "Good bye!".into(),
//     };

//     // Fund dao so we can send the send proposal deposit
//     app.send_tokens(
//         contracts.governance.addr().clone(),
//         Addr::unchecked(my_dao_addr.clone()),
//         &coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
//     )
//     .unwrap();

//     // TODO resigning a core slot does not require a second proposal deposit ...
//     // TODO Create, vote on and execute the dao proposal without the helper to avoid having to send a second proposal deposit

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr,
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();
//     // TODO query core_slots and assert core_tech is empty
//     let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
//     println!("\n\n core_slots {:?}", core_slots);
//     assert_eq!(core_slots.core_tech, None);
// }

// #[test]
// fn set_core_slot_unauthorized() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");

//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

//     println!("\n\n contracts {:#?}", contracts);

//     // Register user identity with valid name

//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();

//     let set_core_slot_err = contracts
//         .governance
//         .set_core_slot(&mut app, &user1, 1)
//         .unwrap_err();

//     assert_eq!(set_core_slot_err, ContractError::Unauthorized {});
// }

// #[test]
// fn improvement_bankmsg() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");

//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());
//     app.update_block(next_block);

//     println!("\n\n contracts {:#?}", contracts);

//     // Register user identity with valid name

//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();
//     app.update_block(next_block);

//     // Register a DAO (required for submitting a proposal)
//     let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());
//     app.update_block(next_block);

//     // Only the CoreSlot DAO can submit an Improvement proposal
//     // So we create a proposal to make my_dao_addr the CoreTech slot

//     // Create a Dao Proposal for a Governance CoreSlot Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
//         title: "Make me CoreTech".into(),
//         description: "Serving the chain".into(),
//         funding: Funding {
//             amount: 10_000_000u128.into(),
//             duration_in_blocks: 3000,
//         },
//         slot: CoreSlot::CoreTech {},
//     });

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     // Vote on and execute the governance proposal
//     gov_vote_helper(
//         &mut app,
//         contracts.clone(),
//         user1.clone(),
//         VoteOption::Yes,
//         user2.clone(),
//         VoteOption::No,
//         1,
//     );

//     // Check that my_dao_addr now has the CoreTech slot
//     let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
//     assert_eq!(core_slots.core_tech.unwrap().dao, my_dao_addr.clone());

//     // Now create the Improvement proposal to send funds

//     // Create a Dao Proposal for Governance Improvement Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Improvement {
//         title: "Send funds".into(),
//         description: "BankMsg".into(),
//         msgs: vec![CosmosMsg::Bank(BankMsg::Send {
//             to_address: user1.clone().into(),
//             amount: vec![Coin {
//                 denom: "ujmes".to_string(),
//                 amount: Uint128::from(GOVERNANCE_INIT_BALANCE - 2 * PROPOSAL_REQUIRED_DEPOSIT),
//             }],
//         })],
//     });

//     // Fund dao so we can send the send proposal deposit
//     app.send_tokens(
//         contracts.governance.addr().clone(),
//         Addr::unchecked(my_dao_addr.clone()),
//         &coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
//     )
//     .unwrap();

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     assert_eq!(
//         app.wrap().query_all_balances(user1.clone()).unwrap(),
//         vec![]
//     );
//     assert_eq!(
//         app.wrap()
//             .query_all_balances(contracts.governance.addr().clone())
//             .unwrap(),
//         coins(
//             GOVERNANCE_INIT_BALANCE - 2 * PROPOSAL_REQUIRED_DEPOSIT,
//             "ujmes"
//         )
//     );

//     // Vote on and execute the governance proposal
//     gov_vote_helper(
//         &mut app,
//         contracts.clone(),
//         user1.clone(),
//         VoteOption::Yes,
//         user2.clone(),
//         VoteOption::No,
//         2,
//     );

//     // Test that the funds were sent from governance to user1
//     assert_eq!(
//         app.wrap().query_all_balances(user1.clone()).unwrap(),
//         coins(
//             GOVERNANCE_INIT_BALANCE - 2 * PROPOSAL_REQUIRED_DEPOSIT,
//             "ujmes"
//         )
//     );
//     assert_eq!(
//         app.wrap()
//             .query_all_balances(contracts.governance.addr().clone())
//             .unwrap(),
//         vec![]
//     );

//     assert_eq!(
//         app.wrap().query_all_balances(BURN_ADDRESS).unwrap(),
//         coins(2 * PROPOSAL_REQUIRED_DEPOSIT, "ujmes")
//     );
// }

// #[test]
// fn improvement_bankmsg_failing() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");

//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

//     // Register user identity with valid name
//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();

//     // Create the flex-multisig dao
//     let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

//     // Only the CoreSlot DAO can submit an Improvement proposal:
//     // So we create a proposal to make my_dao_addr the CoreTech slot

//     // Create a Dao Proposal for a Governance CoreSlot Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::CoreSlot {
//         title: "Make me CoreTech".into(),
//         description: "Serving the chain".into(),
//         funding: Funding {
//             amount: 10_000_000u128.into(),
//             duration_in_blocks: 3000,
//         },
//         slot: CoreSlot::CoreTech {},
//     });

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     // Vote on and execute the governance proposal
//     gov_vote_helper(
//         &mut app,
//         contracts.clone(),
//         user1.clone(),
//         VoteOption::Yes,
//         user2.clone(),
//         VoteOption::No,
//         1,
//     );

//     // Check that my_dao_addr now has the CoreTech slot
//     let core_slots = contracts.governance.query_core_slots(&mut app).unwrap();
//     assert_eq!(core_slots.core_tech.unwrap().dao, my_dao_addr);

//     // Now create the Improvement proposal to send funds

//     // Create a Dao Proposal for Governance Improvement Proposal
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Improvement {
//         title: "Send funds".into(),
//         description: "BankMsg".into(),
//         msgs: vec![CosmosMsg::Bank(BankMsg::Send {
//             to_address: user1.clone().into(),
//             amount: vec![Coin {
//                 denom: "ujmes".to_string(),
//                 amount: Uint128::from(GOVERNANCE_INIT_BALANCE),
//             }],
//         })],
//     });

//     // Fund dao so we can send the send proposal deposit
//     app.send_tokens(
//         contracts.governance.addr().clone(),
//         Addr::unchecked(my_dao_addr.clone()),
//         &coins(PROPOSAL_REQUIRED_DEPOSIT, "ujmes"),
//     )
//     .unwrap();

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr,
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg).unwrap(),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     assert_eq!(
//         app.wrap().query_all_balances(user1.clone()).unwrap(),
//         vec![]
//     );
//     assert_eq!(
//         app.wrap()
//             .query_all_balances(contracts.governance.addr().clone())
//             .unwrap(),
//         coins(
//             GOVERNANCE_INIT_BALANCE - 2 * PROPOSAL_REQUIRED_DEPOSIT,
//             "ujmes"
//         )
//     );

//     // Vote on and execute the governance proposal
//     let proposal_result = gov_vote_helper(
//         &mut app,
//         contracts.clone(),
//         user1.clone(),
//         VoteOption::No,
//         user2.clone(),
//         VoteOption::No,
//         2,
//     );

//     println!("\n\n final_proposal_result {:?}", proposal_result);

//     // Test that deposit was forward to the distribution contract
//     assert_eq!(
//         app.wrap().query_all_balances(user1.clone()).unwrap(),
//         vec![]
//     );
//     assert_eq!(
//         app.wrap()
//             .query_all_balances(contracts.governance.addr().clone())
//             .unwrap(),
//         coins(
//             GOVERNANCE_INIT_BALANCE - 2 * PROPOSAL_REQUIRED_DEPOSIT,
//             "ujmes"
//         )
//     );
//     assert_eq!(
//         app.wrap()
//             .query_all_balances(contracts.governance.addr().clone())
//             .unwrap(),
//         coins(
//             GOVERNANCE_INIT_BALANCE - 2 * PROPOSAL_REQUIRED_DEPOSIT,
//             "ujmes"
//         )
//     );

//     let final_proposal: ProposalResponse = app
//         .wrap()
//         .query_wasm_smart(contracts.governance.addr(), &QueryMsg::Proposal { id: 2 })
//         .unwrap();
//     println!("\n\n final_proposal {:#?}", final_proposal);
//     assert_eq!(final_proposal.status, ProposalStatus::ExpiredConcluded);
//     assert_eq!(
//         app.wrap().query_all_balances(BURN_ADDRESS).unwrap(),
//         coins(2 * PROPOSAL_REQUIRED_DEPOSIT, "ujmes")
//     );
// }

// TODO test as text proposal funding attachment
// #[test]
// fn governance_funding_proposal_passing() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");

//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

//     // Register user identity with valid name
//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();

//     // Create the flex-multisig dao
//     let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

//     // Governance Proposal Msg
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Funding {
//         title: "Funding".to_string(),
//         description: "Give me money".to_string(),
//         duration: FUNDING_DURATION,
//         amount: Uint128::from(FUNDING_AMOUNT),
//     });

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     // Test after proposal execution the deposit is sent to the governance contract
//     assert_eq!(
//         app.wrap()
//             .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
//             .unwrap(),
//         vec![]
//     );

//     let period_info_posting = contracts.governance.query_period_info(&mut app).unwrap();
//     println!("\n\n period_info_posting {:?}", period_info_posting);
//     assert_eq!(period_info_posting.current_period, ProposalPeriod::Posting);

//     // Skip period from Posting to Voting
//     app.update_block(|mut block| {
//         block.time = block
//             .time
//             .plus_seconds(period_info_posting.posting_period_length);
//         block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
//     });

//     assert_eq!(
//         app.wrap()
//             .query_all_balances(contracts.governance.addr().clone())
//             .unwrap(),
//         coins(GOVERNANCE_INIT_BALANCE + PROPOSAL_REQUIRED_DEPOSIT, "ujmes")
//     );

//     let period_info_voting = contracts.governance.query_period_info(&mut app).unwrap();
//     println!("\n\n period_info_voting{:?}", period_info_voting);

//     assert_eq!(
//         period_info_voting,
//         PeriodInfoResponse {
//             current_block: 12355,
//             current_period: ProposalPeriod::Voting,
//             current_time_in_cycle: 60,
//             current_posting_start: 1660000000,
//             current_voting_start: 1660000040,
//             current_voting_end: 1660000080,
//             next_posting_start: 1660000080,
//             next_voting_start: 1660000120,
//             posting_period_length: 40,
//             voting_period_length: 40,
//             cycle_length: 80
//         }
//     );

//     // User1 votes yes to on the governance proposal to pass it

//     let fund_proposal_vote = contracts
//         .governance
//         .vote(&mut app, &user1, 1, VoteOption::Yes)
//         .unwrap();
//     println!("\n\n fund_proposal_vote {:?}", fund_proposal_vote);

//     let proposal_result = contracts.governance.query_proposal(&mut app, 1).unwrap();
//     println!("\n\n proposal_result {:?}", proposal_result);

//     // Test that you can't conclude a proposal in the voting period
//     let voting_not_ended_err = contracts
//         .governance
//         .conclude(&mut app, &user1, 1)
//         .unwrap_err();
//     assert_eq!(voting_not_ended_err, ContractError::VotingPeriodNotEnded {});

//     // Skip period from Voting to Posting so we can conclude the prBLOoKSECNDS
//     app.update_block(|mut block| {
//         block.time = block
//             .time
//             .plus_seconds(period_info_posting.voting_period_length);
//         block.height += period_info_posting.voting_period_length / SECONDS_PER_BLOCK;
//     });

//     let period_info_posting2 = contracts.governance.query_period_info(&mut app).unwrap();
//     println!("\n\n period_info_posting2 {:?}", period_info_posting2);

//     let conclude_proposal_result = contracts.governance.conclude(&mut app, &user1, 1).unwrap();
//     println!(
//         "\n\n conclude_proposal_result {:?}",
//         conclude_proposal_result
//     );

//     // Test that you can't conclude a proposal (and execute its msgs) a second time
//     let conclude2_proposal_result = contracts
//         .governance
//         .conclude(&mut app, &user1, 1)
//         .unwrap_err();
//     assert_eq!(
//         conclude2_proposal_result,
//         ContractError::ProposalAlreadyConcluded {}
//     );
//     println!(
//         "\n\n conclude2_proposal_result {:?}",
//         conclude2_proposal_result
//     );

//     // Skip half the grant duration time to allow us to claim funds
//     app.update_block(|mut block| {
//         block.time = block.time.plus_seconds(FUNDING_DURATION / 2);
//         block.height += FUNDING_DURATION / 2 / SECONDS_PER_BLOCK;
//     });

//     // TODO instead of claiming funds, we receive the funds from L1 with every block

//     assert_eq!(
//         app.wrap()
//             .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
//             .unwrap(),
//         coins(FUNDING_AMOUNT / 2 + PROPOSAL_REQUIRED_DEPOSIT, "ujmes")
//     );

//     // Skip double the grant duration time to claim 100% of the funds
//     app.update_block(|mut block| {
//         block.time = block.time.plus_seconds(FUNDING_DURATION * 2);
//         block.height += FUNDING_DURATION * 2 / SECONDS_PER_BLOCK;
//     });

//     todo!(); // instead of claiming funds, we receive the funds from L1 with every block

//     // let claim_funds_result = contracts
//     //     .distribution
//     //     .claim(&mut app, &Addr::unchecked(my_dao_addr.clone()), 1)
//     //     .unwrap();
//     // println!("\n\n claim_funds_result {:?}", claim_funds_result);

//     assert_eq!(
//         app.wrap()
//             .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
//             .unwrap(),
//         coins(FUNDING_AMOUNT + PROPOSAL_REQUIRED_DEPOSIT, "ujmes")
//     );

//     // Skip period from Posting to VotingBLOKSECNDS
//     app.update_block(|mut block| {
//         block.time = block
//             .time
//             .plus_seconds(period_info_posting.posting_period_length);
//         block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
//     });

//     let period_info_voting = contracts.governance.query_period_info(&mut app).unwrap();
//     println!("\n\n period_info_voting{:?}", period_info_voting);

//     // Test that after conclusion, user2 can no longer vote on the proposal
//     let post_conclusion_vote = contracts
//         .governance
//         .vote(&mut app, &user2, 1, VoteOption::No)
//         .unwrap_err();
//     println!("\n\n post_conclusion_vote {:?}", post_conclusion_vote);

//     assert_eq!(
//         post_conclusion_vote,
//         ContractError::ProposalAlreadyConcluded {}.into()
//     );

//     let post_conclusion_proposal = contracts.governance.query_proposal(&mut app, 1).unwrap();
//     assert_eq!(post_conclusion_proposal.coins_no, Uint128::zero());

//     todo!() // Test that a failing proposal never executes the msgs
// }

// TODO Test failing funding proposal as a textmessage funding attachment
// #[test]
// fn governance_funding_proposal_failing() {
//     let mut app = mock_app();

//     let owner = Addr::unchecked("owner");
//     let user1 = Addr::unchecked("user1");
//     let user2 = Addr::unchecked("user2");

//     let contracts = instantiate_contracts(&mut app, user1.clone(), user2.clone(), owner.clone());

//     // Register user identity with valid name
//     contracts
//         .identityservice
//         .register_user(&mut app, &user1, "user1id".to_string())
//         .unwrap();

//     // Create the flex-multisig dao
//     let my_dao_addr = create_dao(&mut app, contracts.clone(), user1.clone(), user2.clone());

//     // Governance Proposal Msg
//     let proposal_msg = ExecuteMsg::Propose(ProposalMsg::Funding {
//         title: "Funding".to_string(),
//         description: "Give me money".to_string(),
//         duration: FUNDING_DURATION,
//         amount: Uint128::from(FUNDING_AMOUNT),
//     });

//     // Create, vote on and execute the dao proposal
//     DaoMultisigContract::gov_proposal_helper(
//         &mut app,
//         my_dao_addr.clone(),
//         &contracts.governance.addr().clone(),
//         user1.clone(),
//         user2.clone(),
//         to_binary(&proposal_msg),
//         PROPOSAL_REQUIRED_DEPOSIT,
//     )
//     .unwrap();

//     let period_info_posting = contracts.governance.query_period_info(&mut app).unwrap();
//     println!("\n\n period_info_posting {:?}", period_info_posting);
//     assert_eq!(period_info_posting.current_period, ProposalPeriod::Posting);

//     // Skip period from Posting to VotingBLOKSECNDS
//     app.update_block(|mut block| {
//         block.time = block
//             .time
//             .plus_seconds(period_info_posting.posting_period_length);
//         block.height += period_info_posting.posting_period_length / SECONDS_PER_BLOCK;
//     });

//     let period_info_voting = contracts.governance.query_period_info(&mut app).unwrap();
//     println!("\n\n period_info_voting{:#?}", period_info_voting);

//     assert_eq!(
//         period_info_voting,
//         PeriodInfoResponse {
//             current_block: 12355,
//             current_period: ProposalPeriod::Voting,
//             current_time_in_cycle: 60,
//             current_posting_start: 1660000000,
//             current_voting_start: 1660000040,
//             current_voting_end: 1660000080,
//             next_posting_start: 1660000080,
//             next_voting_start: 1660000120,
//             posting_period_length: 40,
//             voting_period_length: 40,
//             cycle_length: 80
//         }
//     );

//     // User1 votes no on the governance proposal to fail it

//     let fund_proposal_vote = contracts
//         .governance
//         .vote(&mut app, &user1, 1, VoteOption::No)
//         .unwrap();
//     println!("\n\n fund_proposal_vote {:?}", fund_proposal_vote);

//     let proposal_result = contracts.governance.query_proposal(&mut app, 1).unwrap();
//     println!("\n\n proposal_result {:?}", proposal_result);

//     // Test that you can't conclude a proposal in the voting period
//     let voting_not_ended_err = contracts
//         .governance
//         .conclude(&mut app, &user1, 1)
//         .unwrap_err();
//     assert_eq!(voting_not_ended_err, ContractError::VotingPeriodNotEnded {});

//     // Skip period from Voting to Posting so we can conclude the prBLOoKSECNDS
//     app.update_block(|mut block| {
//         block.time = block
//             .time
//             .plus_seconds(period_info_posting.voting_period_length);
//         block.height += period_info_posting.voting_period_length / SECONDS_PER_BLOCK;
//     });

//     let period_info_posting2 = contracts.governance.query_period_info(&mut app).unwrap();
//     println!("\n\n period_info_posting2 {:?}", period_info_posting2);

//     let conclude_proposal_result = contracts.governance.conclude(&mut app, &user1, 1).unwrap();
//     println!(
//         "\n\n conclude_proposal_result {:?}",
//         conclude_proposal_result
//     );

//     // Test that you can't conclude a proposal (and execute its msgs) a second time
//     let conclude2_proposal_result = contracts
//         .governance
//         .conclude(&mut app, &user1, 1)
//         .unwrap_err();
//     assert_eq!(
//         conclude2_proposal_result,
//         ContractError::ProposalAlreadyConcluded {}
//     );
//     println!(
//         "\n\n conclude2_proposal_result {:?}",
//         conclude2_proposal_result
//     );

//     // Skip half the grant duration time to allow us to test if the failing proposal lets us claim funds
//     app.update_block(|mut block| {
//         block.time = block.time.plus_seconds(FUNDING_DURATION / 2);
//         block.height += FUNDING_DURATION / 2 / SECONDS_PER_BLOCK;
//     });

//     todo!(); // instead of claiming funds, we receive the funds from L1 with every block

//     assert_eq!(
//         app.wrap()
//             .query_all_balances(Addr::unchecked(my_dao_addr.clone()))
//             .unwrap(),
//         vec![]
//     );
// }

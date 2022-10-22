use std::convert::identity;
use std::vec;

use cosmwasm_std::testing::mock_env;
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::to_binary;
use cosmwasm_std::Addr;
use cosmwasm_std::StdResult;
use cosmwasm_std::Timestamp;
use cosmwasm_std::Uint128;
use cw20::BalanceResponse;
use cw20::Cw20ExecuteMsg;
use cw_multi_test::App;
use cw_multi_test::AppBuilder;
use cw_multi_test::BankKeeper;
use cw_multi_test::{ContractWrapper, Executor};

use bjmes_token::msg::ExecuteMsg as BjmesExecuteMsg;
use bjmes_token::msg::InstantiateMsg as BjmesInstantiateMsg;
use bjmes_token::msg::QueryMsg as BjmesQueryMsg;

use artist_curator::contract::execute as artist_curator_execute;
use artist_curator::contract::instantiate as artist_curator_instantiate;
use artist_curator::contract::query as artist_curator_query;
use artist_curator::msg::ExecuteMsg as ArtistCuratorExecuteMsg;
use artist_curator::msg::InstantiateMsg as ArtistCuratorInstantiateMsg;
use artist_curator::msg::QueryMsg as ArtistCuratorQueryMsg;

use governance::msg::PeriodInfoResponse;
use governance::msg::ProposalResponse;
use governance::msg::ProposalsResponse;
use governance::state::ProposalStatus;
use governance::state::ProposalType;
use governance::state::VoteOption::{No, Yes};

use governance::msg::ExecuteMsg as GovernanceExecuteMsg;
use governance::msg::InstantiateMsg as GovernanceInstantiateMsg;
use governance::msg::QueryMsg as GovernanceQueryMsg;

use identityservice::msg::ExecuteMsg as IdentityserviceExecuteMsg;
use identityservice::msg::InstantiateMsg as IdentityserviceInstantiateMsg;
use identityservice::msg::QueryMsg as IdentityserviceQueryMsg;

use distribution::msg::ExecuteMsg as DistributionExecuteMsg;
use distribution::msg::InstantiateMsg as DistributionInstantiateMsg;
use distribution::msg::QueryMsg as DistributionQueryMsg;

use dao::msg::ExecuteMsg as DaoExecuteMsg;
use dao::msg::QueryMsg as DaoQueryMsg;
use jmes::msg::DaoInstantiateMsg;

const PROPOSAL_REQUIRED_DEPOSIT: u128 = 1000;
const EPOCH_START: u64 = 1660000010;

#[test]
fn test_contract_instantiation() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    // Instantiate needed contracts
    let bjmes_instance = instantiate_bjmes_token(&mut app, owner.clone());
    let governance_instance =
        instantiate_governance(&mut app, owner.clone(), bjmes_instance.clone());

    let dao_code_id = dao_code_id(&mut app);

    let identityservice_instance = instantiate_identityservice(
        &mut app,
        owner.clone(),
        governance_instance.clone(),
        dao_code_id,
    );
    let distribution_instance = instantiate_distribution(
        &mut app,
        owner.clone(),
        governance_instance.clone(),
        identityservice_instance.clone(),
    );

    let art_nft_code_id = art_nft_code_id(&mut app);
    let artist_nft_code_id = artist_nft_code_id(&mut app);

    let artist_curator_instance = instantiate_artist_curator(
        &mut app,
        owner.clone(),
        governance_instance.clone(),
        identityservice_instance.clone(),
        art_nft_code_id,
        artist_nft_code_id,
    );

    println!("\n\ndao_code_id {:?}", dao_code_id);

    println!("\n\nbjmes_instance {:?}", &bjmes_instance);
    println!("\n\ngovernance_instance {:?}", &governance_instance);
    println!(
        "\n\nidentityservice_instance {:?}",
        &identityservice_instance
    );
    println!("\n\ndistribution_instance {:?}", &distribution_instance);

    assert_eq!(bjmes_instance, "contract0");
    assert_eq!(governance_instance, "contract1");
    assert_eq!(identityservice_instance, "contract2");
    assert_eq!(distribution_instance, "contract3");
}

#[test]
fn test_feature_proposal_artist_curator() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    // Instantiate needed contracts
    let bjmes_instance = instantiate_bjmes_token(&mut app, owner.clone());
    let governance_instance =
        instantiate_governance(&mut app, owner.clone(), bjmes_instance.clone());

    let dao_code_id = dao_code_id(&mut app);

    let identityservice_instance = instantiate_identityservice(
        &mut app,
        owner.clone(),
        governance_instance.clone(),
        dao_code_id,
    );
    let distribution_instance = instantiate_distribution(
        &mut app,
        owner.clone(),
        governance_instance.clone(),
        identityservice_instance.clone(),
    );

    let art_nft_code_id = art_nft_code_id(&mut app);
    let artist_nft_code_id = artist_nft_code_id(&mut app);

    let artist_curator_instance = instantiate_artist_curator(
        &mut app,
        owner.clone(),
        governance_instance.clone(),
        identityservice_instance.clone(),
        art_nft_code_id,
        artist_nft_code_id,
    );

    mint_tokens(
        &mut app,
        &user1.clone(),
        &bjmes_instance,
        &user1,
        PROPOSAL_REQUIRED_DEPOSIT * 2,
    );

    check_bjmes_balance(
        &mut app,
        &bjmes_instance,
        &user1,
        PROPOSAL_REQUIRED_DEPOSIT * 2,
    );

    // Query empty proposals
    let resp: ProposalsResponse = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &GovernanceQueryMsg::Proposals {
                start: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        resp,
        ProposalsResponse {
            proposal_count: 0,
            proposals: vec![]
        }
    );
    // Query contract config
    let config: governance::state::Config = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &governance::msg::QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(config.bjmes_token_addr, bjmes_instance);
    assert_eq!(
        config.proposal_required_deposit,
        Uint128::from(config.proposal_required_deposit)
    );

    // Query PeriodInfo: Posting
    let res: PeriodInfoResponse = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &governance::msg::QueryMsg::PeriodInfo {},
        )
        .unwrap();

    assert_eq!(res.current_period, governance::msg::ProposalPeriod::Posting);
    assert_eq!(res.current_time_in_cycle, 10);

    // Skip period from Posting to Voting
    app.update_block(|mut block| {
        block.time = block.time.plus_seconds(config.posting_period_length);
        block.height += config.posting_period_length / 5;
    });

    // Query PeriodInfo: Voting
    let res: PeriodInfoResponse = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &governance::msg::QueryMsg::PeriodInfo {},
        )
        .unwrap();

    assert_eq!(res.current_period, governance::msg::ProposalPeriod::Voting);
    assert_eq!(res.current_time_in_cycle, 10 + config.posting_period_length);

    // Skip period from Voting to Posting
    app.update_block(|mut block| {
        block.time = block.time.plus_seconds(config.voting_period_length);
        block.height += config.posting_period_length / 5;
    });

    // Test valid proposal submission
    let request_feature_msg = Cw20ExecuteMsg::Send {
        contract: governance_instance.to_string(),
        msg: to_binary(&governance::msg::Cw20HookMsg::RequestFeature {
            title: String::from("Artist Curator"),
            description: String::from("Proposal"),
            feature: governance::msg::Feature::ArtistCurator {
                approved: 2,
                duration: 300,
            },
        })
        .unwrap(),
        amount: Uint128::from(PROPOSAL_REQUIRED_DEPOSIT),
    };

    let _resp = app
        .execute_contract(
            user1.clone(),
            bjmes_instance.clone(),
            &request_feature_msg,
            &[],
        )
        .unwrap();

    let resp: ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &governance::msg::QueryMsg::Proposal { id: 1 },
        )
        .unwrap();
    assert_eq!(
        resp,
        ProposalResponse {
            id: 1,
            dao: user1.clone(),
            title: "Artist Curator".to_string(),
            description: "Proposal".to_string(),
            prop_type: ProposalType::FeatureRequest(governance::msg::Feature::ArtistCurator {
                approved: 2,
                duration: 300
            }),
            coins_yes: Uint128::zero(),
            coins_no: Uint128::zero(),
            yes_voters: vec![],
            no_voters: vec![],
            deposit_amount: Uint128::from(1000u128),
            start_block: 132345,
            posting_start: 1660906864,
            voting_start: 1661206864,
            voting_end: 1661813728,
            concluded: false,
            status: ProposalStatus::Posted
        }
    );

    let resp: ProposalsResponse = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &governance::msg::QueryMsg::Proposals {
                start: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(
        resp,
        ProposalsResponse {
            proposal_count: 1,
            proposals: vec![ProposalResponse {
                id: 1,
                dao: user1.clone(),
                title: "Artist Curator".to_string(),
                description: "Proposal".to_string(),
                prop_type: ProposalType::FeatureRequest(governance::msg::Feature::ArtistCurator {
                    approved: 2,
                    duration: 300
                }),
                coins_yes: Uint128::zero(),
                coins_no: Uint128::zero(),
                yes_voters: vec![],
                no_voters: vec![],
                deposit_amount: Uint128::from(1000u128),
                start_block: 132345,
                posting_start: 1660906864,
                voting_start: 1661206864,
                voting_end: 1661813728,
                concluded: false,
                status: ProposalStatus::Posted
            }]
        }
    );

    // Query bJMES token balance after proposal submission
    let msg = BjmesQueryMsg::Balance {
        address: user1.clone().to_string(),
    };
    let resp: StdResult<BalanceResponse> =
        app.wrap().query_wasm_smart(bjmes_instance.clone(), &msg);

    assert_eq!(
        resp.unwrap().balance,
        Uint128::from(config.proposal_required_deposit)
    );

    // TODO test vote with no coins

    // Test proposal vote in posting period
    let vote_msg = governance::msg::ExecuteMsg::Vote { id: 1, vote: Yes };

    let err = app
        .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
        .unwrap_err();

    assert_eq!(err.root_cause().to_string(), "NotVotingPeriod");

    // Skip period from Posting to Voting
    app.update_block(|mut block| {
        block.time = block.time.plus_seconds(config.posting_period_length);
        block.height += config.posting_period_length / 5;
    });

    // Query PeriodInfo: Voting
    let res: PeriodInfoResponse = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &governance::msg::QueryMsg::PeriodInfo {},
        )
        .unwrap();

    assert_eq!(res.current_period, governance::msg::ProposalPeriod::Voting);
    assert_eq!(res.current_time_in_cycle, 10 + config.posting_period_length);

    // Test proposal yes vote
    let vote_msg = governance::msg::ExecuteMsg::Vote { id: 1, vote: Yes };

    let _resp = app
        .execute_contract(user1.clone(), governance_instance.clone(), &vote_msg, &[])
        .unwrap();

    let resp: ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            governance_instance.clone(),
            &governance::msg::QueryMsg::Proposal { id: 1 },
        )
        .unwrap();

    assert_eq!(
        resp,
        ProposalResponse {
            id: 1,
            dao: user1.clone(),
            title: "Artist Curator".to_string(),
            description: "Proposal".to_string(),
            prop_type: ProposalType::FeatureRequest(governance::msg::Feature::ArtistCurator {
                approved: 2,
                duration: 300
            }),
            coins_yes: Uint128::from(1000u128),
            coins_no: Uint128::zero(),
            yes_voters: vec![user1.clone()],
            no_voters: vec![],
            deposit_amount: Uint128::from(1000u128),
            start_block: 132345,
            posting_start: 1660906864,
            voting_start: 1661206864,
            voting_end: 1661813728,
            concluded: false,
            status: ProposalStatus::Voting
        }
    );
}

fn mock_app() -> App {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();

    AppBuilder::new()
        .with_api(api)
        .with_block(env.block)
        .with_bank(bank)
        .with_storage(storage)
        .build(|_, _, _| {})
}

fn instantiate_bjmes_token(app: &mut App, owner: Addr) -> Addr {
    let bjmes_code = ContractWrapper::new(
        bjmes_token::contract::execute,
        bjmes_token::contract::instantiate,
        bjmes_token::contract::query,
    );

    let bjmes_code_id = app.store_code(Box::new(bjmes_code));

    let init_msg = &BjmesInstantiateMsg {
        name: "bonded JMES".to_string(),
        symbol: "bjmes".to_string(),
        decimals: 10,
        initial_balances: vec![],
        marketing: None,
        mint: None,
    };

    app.instantiate_contract(bjmes_code_id, owner, init_msg, &[], "bonded JMES", None)
        .unwrap()
}

fn instantiate_governance(app: &mut App, owner: Addr, bjmes_token_addr: Addr) -> Addr {
    let governance_code = ContractWrapper::new(
        governance::contract::execute,
        governance::contract::instantiate,
        governance::contract::query,
    );

    let governance_code_id = app.store_code(Box::new(governance_code));

    let init_msg = &GovernanceInstantiateMsg {
        owner: owner.to_string(),
        bjmes_token_addr: bjmes_token_addr.to_string(),
        artist_curator_addr: None,
        proposal_required_deposit: Uint128::new(1000u128),
        proposal_required_percentage: 51,
        period_start_epoch: 1660000000,
        posting_period_length: 300000,
        voting_period_length: 606864,
    };

    app.instantiate_contract(governance_code_id, owner, init_msg, &[], "governance", None)
        .unwrap()
}

fn dao_code_id(app: &mut App) -> u64 {
    let dao_code = ContractWrapper::new(
        dao::contract::execute,
        dao::contract::instantiate,
        dao::contract::query,
    );
    app.store_code(Box::new(dao_code))
}

fn instantiate_dao(
    app: &mut App,
    dao_code_id: u64,
    owner: Addr,
    init_msg: DaoInstantiateMsg,
) -> Addr {
    app.instantiate_contract(dao_code_id, owner, &init_msg, &[], "dao", None)
        .unwrap()
}

fn instantiate_identityservice(
    app: &mut App,
    sender: Addr,
    governance_addr: Addr,
    dao_code_id: u64,
) -> Addr {
    let identityservice_code = ContractWrapper::new(
        identityservice::contract::execute,
        identityservice::contract::instantiate,
        identityservice::contract::query,
    );

    let identityservice_code_id = app.store_code(Box::new(identityservice_code));

    let init_msg = &IdentityserviceInstantiateMsg {
        owner: governance_addr,
        dao_code_id,
    };

    app.instantiate_contract(
        identityservice_code_id,
        sender,
        init_msg,
        &[],
        "identityservice",
        None,
    )
    .unwrap()
}

fn instantiate_distribution(
    app: &mut App,
    sender: Addr,
    governance_addr: Addr,
    identityservice_addr: Addr,
) -> Addr {
    let distribution_code = ContractWrapper::new(
        distribution::contract::execute,
        distribution::contract::instantiate,
        distribution::contract::query,
    );

    let distribution_code_id = app.store_code(Box::new(distribution_code));

    let init_msg = &DistributionInstantiateMsg {
        owner: governance_addr,
        identityservice_contract: identityservice_addr,
    };

    app.instantiate_contract(
        distribution_code_id,
        sender,
        init_msg,
        &[],
        "distribution",
        None,
    )
    .unwrap()
}

fn instantiate_artist_curator(
    app: &mut App,
    sender: Addr,
    governance_addr: Addr,
    identityservice_addr: Addr,
    art_nft_code_id: u64,
    artist_nft_code_id: u64,
) -> Addr {
    let artist_curator_code = ContractWrapper::new(
        artist_curator::contract::execute,
        artist_curator::contract::instantiate,
        artist_curator::contract::query,
    )
    .with_reply(artist_curator::contract::reply);

    let artist_curator_code_id = app.store_code(Box::new(artist_curator_code));

    let init_msg = &ArtistCuratorInstantiateMsg {
        owner: governance_addr,
        identityservice_contract: identityservice_addr,
        art_nft_name: "Art NFT".to_string(),
        art_nft_symbol: "artnft".to_string(),
        art_nft_code_id,
        artist_nft_name: "Artist NFT".to_string(),
        artist_nft_symbol: "artistnft".to_string(),
        artist_nft_code_id,
    };

    app.instantiate_contract(
        artist_curator_code_id,
        sender,
        init_msg,
        &[],
        "ArtistCurator",
        None,
    )
    .unwrap()
}

fn artist_nft_code_id(app: &mut App) -> u64 {
    let artist_nft_code = ContractWrapper::new(
        artist_nft::entry::execute,
        artist_nft::entry::instantiate,
        artist_nft::entry::query,
    );
    app.store_code(Box::new(artist_nft_code))
}

fn art_nft_code_id(app: &mut App) -> u64 {
    let art_nft_code = ContractWrapper::new(
        art_nft::entry::execute,
        art_nft::entry::instantiate,
        art_nft::entry::query,
    );
    app.store_code(Box::new(art_nft_code))
}

fn mint_tokens(app: &mut App, minter: &Addr, contract_addr: &Addr, recipient: &Addr, amount: u128) {
    let msg = &Cw20ExecuteMsg::Mint {
        recipient: recipient.to_string(),
        amount: Uint128::new(amount),
    };

    app.execute_contract(minter.clone(), contract_addr.clone(), msg, &[])
        .unwrap();
}

fn check_bjmes_balance(app: &mut App, token: &Addr, address: &Addr, expected: u128) {
    let msg = BjmesQueryMsg::Balance {
        address: address.clone().to_string(),
    };
    let res: StdResult<BalanceResponse> = app.wrap().query_wasm_smart(token, &msg);
    println!("\n\nres {:?}", res);
    assert_eq!(res.unwrap().balance, Uint128::from(expected));
}

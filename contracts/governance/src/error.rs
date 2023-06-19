use cosmwasm_std::{Decimal, OverflowError, StdError};
use cw_utils::Threshold;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("WrongCoreTeamMemberCount (Core Team must have between {min} and {max} members)!")]
    WrongCoreTeamMemberCount { min: usize, max: usize },
    #[error("WrongCoreTeamMemberVotingPower (Each Core Team must have less than {threshold:?} but one members has {current} voting power)!")]
    WrongCoreTeamMemberVotingPower { threshold: Threshold, current: u64 },
    #[error("InsufficientProposalFee ({proposal_fee} JMES fee required to post a proposal)!")]
    InsufficientProposalFee { proposal_fee: u128 },
    #[error("NoVoteCoins ({min_vote_coins}  bJMES required to vote)!")]
    NoVoteCoins { min_vote_coins: u128 },
    #[error("InsufficientVoteCoins ({min_vote_coins} bJMES required to vote)!")]
    InsufficientVoteCoins { min_vote_coins: u128 },
    #[error("User already voted!")]
    UserAlreadyVoted {},
    #[error("ProposalNotActive")]
    ProposalNotActive {},
    #[error("NotPostingPeriod")]
    NotPostingPeriod {},
    #[error("NotVotingPeriod")]
    NotVotingPeriod {},
    #[error("TooLateToChallengeCoreSlot proposal must be submitted during the first half of the posting period!")]
    TooLateToChallengeCoreSlot {},
    #[error("VotingPeriodNotEnded")]
    VotingPeriodNotEnded,
    #[error("ProposalNotValid {error} ")]
    ProposalNotValid { error: String },
    #[error("ProposalAlreadyConcluded")]
    ProposalAlreadyConcluded {},
    #[error("ProposalVotingEnded")]
    ProposalVotingEnded {},
    #[error("InvalidProposalType")]
    InvalidProposalType {},
    #[error("WrongDao")]
    WrongDao {},
    #[error("AlreadyHoldingCoreSlot")]
    AlreadyHoldingCoreSlot {},
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}

use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Insufficient token deposit!")]
    InsufficientProposalFee {},
    #[error("InsufficientProposalFee (10 JMES fee required to post a proposal)!")]
    InsufficientDeposit {},
    #[error("NoVoteCoins (1000 bJMES required to vote)!")]
    NoVoteCoins {},
    #[error("InsufficientVoteCoins (1000 bJMES required to vote)!")]
    InsufficientVoteCoins {},
    #[error("User already voted!")]
    UserAlreadyVoted {},
    #[error("ProposalNotActive")]
    ProposalNotActive {},
    #[error("NotPostingPeriod")]
    NotPostingPeriod {},
    #[error("NotVotingPeriod")]
    NotVotingPeriod {},
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

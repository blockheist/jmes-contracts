use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Insufficient token deposit!")]
    InsufficientDeposit {},
    #[error("NoVoteCoins!")]
    NoVoteCoins {},
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
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}

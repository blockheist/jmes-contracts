use cosmwasm_std::{OverflowError, StdError};
use cw_utils::Threshold;
use thiserror::Error;

use cw_controllers::{AdminError, HookError};

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Hook(#[from] HookError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Message contained duplicate member: {member}")]
    DuplicateMember { member: String },

    #[error("A maximum of {max} members are allowed, actual: {actual}")]
    TooManyMembers { max: usize, actual: usize },

    #[error("WrongCoreTeamMemberCount (Core Team must have between {min} and {max} members)!")]
    WrongCoreTeamMemberCount { min: usize, max: usize },

    #[error("InvalidThresholdPercentage max is 100 (current {current})")]
    InvalidThresholdPercentage { current: u64 },

    #[error("InvalidTotalVotingPercentage max is 100 (current {current})")]
    InvalidTotalVotingPercentage { current: u64 },

    #[error("WrongCoreTeamMemberVotingPower (Each Core Team must have less than {threshold:?} but one member has {current} voting power)!")]
    WrongCoreTeamMemberVotingPower { threshold: Threshold, current: u64 },
}

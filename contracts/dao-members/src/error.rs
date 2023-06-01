use cosmwasm_std::{OverflowError, StdError};
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

    #[error("WrongMemberCount (Core Team must have between {min} and {max} members)!")]
    WrongCoreTeamMemberCount { min: usize, max: usize },
}

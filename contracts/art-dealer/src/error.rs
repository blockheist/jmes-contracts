use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("SoldOut")]
    SoldOut {},

    #[error("UnauthorizedTokenContract")]
    UnauthorizedTokenContract {},

    #[error("Uninitialized")]
    Uninitialized {},

    #[error("WrongPaymentAmount")]
    WrongPaymentAmount {},

    #[error("InvalidTokenReplyId")]
    InvalidTokenReplyId {},

    #[error("Cw721NotLinked")]
    Cw721NotLinked {},

    #[error("Cw721AlreadyLinked")]
    Cw721AlreadyLinked {},

    #[error("ApprovedExceeded")]
    ApprovedExceeded {},

    #[error("ApprovalExpired")]
    ApprovalExpired {},

    #[error("DealerNotApproved")]
    DealerNotApproved {},
}

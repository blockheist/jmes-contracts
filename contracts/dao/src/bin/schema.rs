use cosmwasm_schema::write_api;

use dao::msg::{ExecuteMsg, QueryMsg};
use jmes::msg::DaoInstantiateMsg as InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}

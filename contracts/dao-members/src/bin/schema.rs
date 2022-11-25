use cosmwasm_schema::write_api;

use dao_members::msg::{DaoMembersInstantiateMsg, ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: DaoMembersInstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}

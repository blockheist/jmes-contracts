use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
// use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use governance::msg::*;
use jmes::msg::GovernanceCoreSlotsResponse as CoreSlotsResponse;
use jmes::msg::GovernanceQueryMsg as QueryMsg;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema_with_title(&schema_for!(InstantiateMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&schema_for!(ExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema_with_title(&schema_for!(QueryMsg), &out_dir, "QueryMsg");
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(PeriodInfoResponse), &out_dir);
    export_schema(&schema_for!(ProposalResponse), &out_dir);
    export_schema(&schema_for!(ProposalsResponse), &out_dir);
    export_schema(&schema_for!(ProposalMsg), &out_dir);
    export_schema_with_title(
        &schema_for!(CoreSlotsResponse),
        &out_dir,
        "CoreSlotsResponse",
    );
    export_schema(&schema_for!(WinningGrantsResponse), &out_dir);
}

use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use ultra_role_provider::msg::{
    ExecuteMsg, HasAnyRoleResponse, InstantiateMsg, QueryMsg, Role, RoleAddressResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg<Role>), &out_dir);
    export_schema(&schema_for!(Role), &out_dir);
    export_schema(&schema_for!(HasAnyRoleResponse), &out_dir);
    export_schema(&schema_for!(RoleAddressResponse), &out_dir);
}

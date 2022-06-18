use crate::{
    contract::{instantiate, NATIVE_JUNO_DENOM},
    msg::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg, SudoMsg},
    ContractError,
};

use cosmwasm_std::{Addr, Empty, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

const SOME: &str = "someone";
const OWNER: &str = "owner";
const BO: &str = "borrower-operations";
const TM: &str = "trove-manager";
const SP: &str = "stability-pool";
const DP: &str = "default-pool";

fn active_pool_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_sudo(crate::sudo::sudo);
    Box::new(contract)
}

fn instantiate_active_pool(app: &mut App, msg: InstantiateMsg) -> Addr {
    let code_id = app.store_code(active_pool_contract());
    app.instantiate_contract(
        code_id,
        Addr::unchecked(SOME),
        &msg,
        &[],
        "active pool",
        None,
    )
    .unwrap()
}

#[test]
fn test_instantiate() {
    let mut app = App::default();

    let msg = InstantiateMsg {
        name: String::from("Active Pool"),
        owner: OWNER.to_string(),
    };

    let active_pool_addr = instantiate_active_pool(&mut app, msg);

    let response: ParamsResponse = app
        .wrap()
        .query_wasm_smart(&active_pool_addr, &QueryMsg::GetParams {})
        .unwrap();

    assert_eq!(
        response.owner, Addr::unchecked(OWNER)
    );
}

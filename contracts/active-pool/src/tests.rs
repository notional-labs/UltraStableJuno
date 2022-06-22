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

    assert_eq!(response.owner, Addr::unchecked(OWNER));

    assert_eq!(response.name, "Active Pool");
}

#[test]
fn test_set_addresses() {
    let mut app = App::default();

    let msg = InstantiateMsg {
        name: String::from("Active Pool"),
        owner: OWNER.to_string(),
    };

    let active_pool_addr = instantiate_active_pool(&mut app, msg);

    let set_addresses_msg = ExecuteMsg::SetAddresses {
        borrower_operations_address: BO.to_string(),
        default_pool_address: DP.to_string(),
        stability_pool_address: SP.to_string(),
        trove_manager_address: TM.to_string(),
    };

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(SOME),
            active_pool_addr.clone(),
            &set_addresses_msg,
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::UnauthorizedOwner {});

    app.execute_contract(
        Addr::unchecked(OWNER),
        active_pool_addr.clone(),
        &set_addresses_msg,
        &[],
    )
    .unwrap();

    let bo_address: Addr = app
        .wrap()
        .query_wasm_smart(
            active_pool_addr.clone(),
            &QueryMsg::GetBorrowerOperationsAddress {},
        )
        .unwrap();
    assert_eq!(bo_address, Addr::unchecked(BO));

    let tm_address: Addr = app
        .wrap()
        .query_wasm_smart(
            active_pool_addr.clone(),
            &QueryMsg::GetTroveManagerAddress {},
        )
        .unwrap();
    assert_eq!(tm_address, Addr::unchecked(TM));

    let sp_address: Addr = app
        .wrap()
        .query_wasm_smart(
            active_pool_addr.clone(),
            &QueryMsg::GetStabilityPoolAddress {},
        )
        .unwrap();
    assert_eq!(sp_address, Addr::unchecked(SP));

    let dp_address: Addr = app
        .wrap()
        .query_wasm_smart(
            active_pool_addr.clone(),
            &QueryMsg::GetDefaultPoolAddress {},
        )
        .unwrap();
    assert_eq!(dp_address, Addr::unchecked(DP));
}

#[test]
fn test_increase_decrease_usj_debt() {
    let mut app = App::default();
    let msg = InstantiateMsg {
        name: String::from("Active Pool"),
        owner: OWNER.to_string(),
    };

    let active_pool_addr = instantiate_active_pool(&mut app, msg);

    let set_addresses_msg = ExecuteMsg::SetAddresses {
        borrower_operations_address: BO.to_string(),
        default_pool_address: DP.to_string(),
        stability_pool_address: SP.to_string(),
        trove_manager_address: TM.to_string(),
    };

    app.execute_contract(
        Addr::unchecked(OWNER),
        active_pool_addr.clone(),
        &set_addresses_msg,
        &[],
    )
    .unwrap();

    let increase_usj_debt_msg = ExecuteMsg::IncreaseUSJDebt {
        amount: Uint128::new(100u128),
    };

    let decrease_usj_debt_msg = ExecuteMsg::DecreaseUSJDebt {
        amount: Uint128::new(50u128),
    };

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(SOME),
            active_pool_addr.clone(),
            &increase_usj_debt_msg,
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::CallerIsNeitherBONorTM {});

    app.execute_contract(
        Addr::unchecked(TM),
        active_pool_addr.clone(),
        &increase_usj_debt_msg,
        &[],
    )
    .unwrap();

    let usj_debt: Uint128 = app
        .wrap()
        .query_wasm_smart(active_pool_addr.clone(), &QueryMsg::GetUSJDebt {})
        .unwrap();

    assert_eq!(usj_debt, Uint128::new(100u128));

    app.execute_contract(
        Addr::unchecked(TM),
        active_pool_addr.clone(),
        &decrease_usj_debt_msg,
        &[],
    )
    .unwrap();

    let usj_debt: Uint128 = app
        .wrap()
        .query_wasm_smart(active_pool_addr.clone(), &QueryMsg::GetUSJDebt {})
        .unwrap();

    assert_eq!(usj_debt, Uint128::new(50u128));
}

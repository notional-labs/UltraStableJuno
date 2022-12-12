use crate::{
    contract::{instantiate, NATIVE_JUNO_DENOM},
    ContractError,
};

use ultra_base::active_pool::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg, SudoMsg};
use anyhow::Result;
use cosmwasm_std::{Addr, Empty, Uint128, coin};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, AppResponse};

const SOME: &str = "someone";
const OWNER: &str = "owner";
const IMPOSTER: &str = "imposter";
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

fn role_provider_contract() -> Box<dyn Contract<Empty>>{
    let contract = ContractWrapper::new(
        ultra_role_provider::contract::execute, 
        ultra_role_provider::contract::instantiate, 
        ultra_role_provider::contract::query
    );
    Box::new(contract)
}

fn instantiate_active_pool(app: &mut App, msg: InstantiateMsg) -> Addr {
    let code_id = app.store_code(active_pool_contract());
    app.instantiate_contract(
        code_id,
        Addr::unchecked(OWNER),
        &msg,
        &[],
        "active pool",
        None,
    )
    .unwrap()
}

fn instantiate_role_provider(app: &mut App, msg: ultra_base::role_provider::InstantiateMsg) -> Addr {
    let code_id = app.store_code(role_provider_contract());
    app.instantiate_contract(
        code_id, 
        Addr::unchecked(OWNER), 
        &msg, 
        &[], 
        "role provider", 
        None
    ).unwrap()
}

fn update_admin_is_ok(app: &mut App, contract_addr: &Addr, sender: &str, new_admin: &str) ->  bool {
    let update_admin_msg = ultra_base::active_pool::ExecuteMsg::UpdateAdmin { 
        admin: Addr::unchecked(new_admin) 
    };
    app
        .execute_contract(
            Addr::unchecked(sender),
            contract_addr.clone(),
            &update_admin_msg,
            &[],
        ).is_ok()
}

fn update_role_is_ok(app: &mut App, contract_addr: &Addr, sender: &str, role_provider: &Addr) -> bool{
    let update_role_msg = ultra_base::active_pool::ExecuteMsg::UpdateRole { 
        role_provider: role_provider.clone() 
    };

    app
        .execute_contract(
            Addr::unchecked(sender), 
            contract_addr.clone(), 
            &update_role_msg, 
            &[]
    ).is_ok()
}

fn increase_ultra_debt(app: &mut App, contract_addr: &Addr, sender: &str, amount: Uint128) -> Result<AppResponse>{
    let increase_ultra_debt_msg = ultra_base::active_pool::ExecuteMsg::IncreaseULTRADebt { 
        amount
    };

    app
        .execute_contract(
            Addr::unchecked(sender), 
            contract_addr.clone(), 
            &increase_ultra_debt_msg, 
            &[]
    )
}

fn decrease_ultra_debt(app: &mut App, contract_addr: &Addr, sender: &str, amount: Uint128) -> Result<AppResponse>{
    let decrease_ultra_debt_msg = ultra_base::active_pool::ExecuteMsg::DecreaseULTRADebt { 
        amount
    };

    app
        .execute_contract(
            Addr::unchecked(sender), 
            contract_addr.clone(), 
            &decrease_ultra_debt_msg, 
            &[]
    )
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
fn test_role_and_admin(){
    let mut app = App::default();

    let msg = InstantiateMsg {
        name: String::from("Active Pool"),
        owner: OWNER.to_string(),
    };

    let active_pool_addr = instantiate_active_pool(&mut app, msg);

    let msg = ultra_base::role_provider::InstantiateMsg{
        active_pool: active_pool_addr.to_string(),
        trove_manager: TM.to_string(),
        owner: OWNER.to_string(),
        stability_pool: SP.to_string(),
        borrower_operations: BO.to_string(),
    };
    let role_provider_addr = instantiate_role_provider(&mut app, msg);

    // update admin
    assert!(update_admin_is_ok(&mut app, &active_pool_addr, OWNER, SOME));
    assert!(!update_admin_is_ok(&mut app, &active_pool_addr, IMPOSTER, OWNER));
    update_admin_is_ok(&mut app, &active_pool_addr, SOME, OWNER);

    // update role
    assert!(update_role_is_ok(&mut app, &active_pool_addr, OWNER, &role_provider_addr));
    assert!(!update_role_is_ok(&mut app, &active_pool_addr, IMPOSTER, &role_provider_addr));

    // increase ultra debt
    let res = increase_ultra_debt(
        &mut app, 
        &active_pool_addr, 
        BO, 
        Uint128::from(1000u128));
    assert!(res.is_ok());

    let res = increase_ultra_debt(
        &mut app, 
        &active_pool_addr, 
        TM, 
        Uint128::from(5000u128));
    assert!(res.is_ok());

    let res = increase_ultra_debt(
        &mut app, 
        &active_pool_addr, 
        IMPOSTER, 
        Uint128::from(5000u128));
    println!("{}",res.unwrap_err());

    // decrease ultra debt
    let res = decrease_ultra_debt(
        &mut app, 
        &active_pool_addr, 
        BO, 
        Uint128::from(1000u128));
    assert!(res.is_ok());

    let res = decrease_ultra_debt(
        &mut app, 
        &active_pool_addr, 
        TM, 
        Uint128::from(1000u128));
    assert!(res.is_ok());

    let res = decrease_ultra_debt(
        &mut app, 
        &active_pool_addr, 
        SP, 
        Uint128::from(1000u128));
    assert!(res.is_ok());

    let res = decrease_ultra_debt(
        &mut app, 
        &active_pool_addr, 
        IMPOSTER, 
        Uint128::from(1000u128));
    println!("{}",res.unwrap_err());
}

#[test]
fn test_increase_decrease_ultra_debt() {
    let mut app = App::default();
    let msg = InstantiateMsg {
        name: String::from("Active Pool"),
        owner: OWNER.to_string(),
    };

    let active_pool_addr = instantiate_active_pool(&mut app, msg);

    // let set_addresses_msg = ExecuteMsg::SetAddresses {
    //     borrower_operations_address: BO.to_string(),
    //     default_pool_address: DP.to_string(),
    //     stability_pool_address: SP.to_string(),
    //     trove_manager_address: TM.to_string(),
    // };

    // app.execute_contract(
    //     Addr::unchecked(OWNER),
    //     active_pool_addr.clone(),
    //     &set_addresses_msg,
    //     &[],
    // )
    // .unwrap();

    let increase_ultra_debt_msg = ExecuteMsg::IncreaseULTRADebt {
        amount: Uint128::new(100u128),
    };

    let decrease_ultra_debt_msg = ExecuteMsg::DecreaseULTRADebt {
        amount: Uint128::new(50u128),
    };

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(SOME),
            active_pool_addr.clone(),
            &increase_ultra_debt_msg,
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::CallerIsNeitherBONorTM {});

    app.execute_contract(
        Addr::unchecked(TM),
        active_pool_addr.clone(),
        &increase_ultra_debt_msg,
        &[],
    )
    .unwrap();

    let ultra_debt: Uint128 = app
        .wrap()
        .query_wasm_smart(active_pool_addr.clone(), &QueryMsg::GetULTRADebt {})
        .unwrap();

    assert_eq!(ultra_debt, Uint128::new(100u128));

    app.execute_contract(
        Addr::unchecked(TM),
        active_pool_addr.clone(),
        &decrease_ultra_debt_msg,
        &[],
    )
    .unwrap();

    let ultra_debt: Uint128 = app
        .wrap()
        .query_wasm_smart(active_pool_addr.clone(), &QueryMsg::GetULTRADebt {})
        .unwrap();

    assert_eq!(ultra_debt, Uint128::new(50u128));
}

use crate::asset::{AssetInfo, PoolInfo};

use wasmswap::msg::{QueryMsg as WasmSwapMsg, InfoResponse};
use cosmwasm_std::{
    Addr, AllBalanceResponse, BankQuery, Coin, QuerierWrapper, QueryRequest, StdResult,
    Uint128,
};

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse, Denom};

const NATIVE_TOKEN_PRECISION: u8 = 6;

/// Returns a native token's balance for a specific account.
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: impl Into<String>,
    denom: impl Into<String>,
) -> StdResult<Uint128> {
    querier
        .query_balance(account_addr, denom)
        .map(|coin| coin.amount)
}

/// Returns the total balances for all coins at a specified account address.
pub fn query_all_balances(querier: &QuerierWrapper, account_addr: Addr) -> StdResult<Vec<Coin>> {
    let all_balances: AllBalanceResponse =
        querier.query(&QueryRequest::Bank(BankQuery::AllBalances {
            address: String::from(account_addr),
        }))?;
    Ok(all_balances.amount)
}

/// Returns a cw20 token balance for an account.
pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: impl Into<String>,
    account_addr: impl Into<String>,
) -> StdResult<Uint128> {
    let resp: Cw20BalanceResponse = querier
        .query_wasm_smart(
            contract_addr,
            &Cw20QueryMsg::Balance {
                address: account_addr.into(),
            },
        )
        .unwrap_or_else(|_| Cw20BalanceResponse {
            balance: Uint128::zero(),
        });

    Ok(resp.balance)
}

/// Returns a cw20 token's symbol.
pub fn query_token_symbol(
    querier: &QuerierWrapper,
    contract_addr: impl Into<String>,
) -> StdResult<String> {
    let res: TokenInfoResponse =
        querier.query_wasm_smart(contract_addr, &Cw20QueryMsg::TokenInfo {})?;

    Ok(res.symbol)
}

/// Returns the total supply of a specific cw20 token.
pub fn query_supply(
    querier: &QuerierWrapper,
    contract_addr: impl Into<String>,
) -> StdResult<Uint128> {
    let res: TokenInfoResponse =
        querier.query_wasm_smart(contract_addr, &Cw20QueryMsg::TokenInfo {})?;

    Ok(res.total_supply)
}

/// Returns the number of decimals that a token (native or cw20 token) has.
pub fn query_token_precision(querier: &QuerierWrapper, asset_info: &AssetInfo) -> StdResult<u8> {
    let decimals = match asset_info {
        AssetInfo::NativeToken { .. } => NATIVE_TOKEN_PRECISION,
        AssetInfo::Cw20Token { contract_addr } => {
            let res: TokenInfoResponse =
                querier.query_wasm_smart(contract_addr, &Cw20QueryMsg::TokenInfo {})?;

            res.decimals
        }
    };

    Ok(decimals)
}

/// Returns JunoSwap pool information.
pub fn query_pool_info(
    querier: &QuerierWrapper,
    pool_contract_addr: String,
) -> StdResult<PoolInfo> {
    let pool_info: InfoResponse = querier.query_wasm_smart(
        pool_contract_addr.clone(),
        &WasmSwapMsg::Info {}
    )?;

    let token1_denom: AssetInfo = match pool_info.token1_denom {
        Denom::Native(denom) => {
            AssetInfo::NativeToken { denom }
        },
        Denom::Cw20(contract_addr) => {
            AssetInfo::Cw20Token { contract_addr }
        }
    };

    let token2_denom: AssetInfo = match pool_info.token2_denom {
        Denom::Native(denom) => {
            AssetInfo::NativeToken { denom }
        },
        Denom::Cw20(contract_addr) => {
            AssetInfo::Cw20Token { contract_addr }
        }
    };

    let res = PoolInfo {
        token1_reserve: pool_info.token1_reserve,
        token1_denom: token1_denom,
        token2_reserve: pool_info.token2_reserve,
        token2_denom: token2_denom,
        pool_contract_addr: pool_contract_addr,
        lp_token_address: pool_info.lp_token_address,
        lp_token_supply: pool_info.lp_token_supply
    };
    Ok(res)
}

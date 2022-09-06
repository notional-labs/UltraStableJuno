use crate::active_pool::QueryMsg as ActivePoolQueryMsg;
use crate::asset::{AssetInfo, PoolInfo};
use crate::default_pool::QueryMsg as DefaultPoolQueryMsg;
use crate::ultra_math::{self, DECIMAL_PRECISION};
use crate::{price_feed, trove_manager};

use cosmwasm_std::{
    Addr, AllBalanceResponse, BankQuery, Coin, Decimal256, QuerierWrapper, QueryRequest, StdError,
    StdResult, Uint128, Uint256,
};
use wasmswap::msg::{InfoResponse, QueryMsg as WasmSwapMsg};

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, Denom, TokenInfoResponse};

const NATIVE_TOKEN_PRECISION: u8 = 6;

// Minimum collateral ratio for individual troves
pub const MCR: Decimal256 = Decimal256::new(Uint256::from_u128(1_100_000_000_000_000_000u128));

// Critical system collateral ratio. If the system's total collateral ratio (TCR) falls below the CCR, Recovery Mode is triggered.
pub const CCR: Decimal256 = Decimal256::new(Uint256::from_u128(1_500_000_000_000_000_000u128));

// Minimum amount of net ULTRA debt a trove must have
pub const MIN_NET_DEBT: Uint128 = Uint128::new(2000u128);

pub const BORROWING_FEE_FLOOR: Decimal256 =
    Decimal256::new(Uint256::from_u128(5_000_000_000_000_000u128)); // 0.5%

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
pub fn query_pool_info(querier: &QuerierWrapper, pool_contract_addr: Addr) -> StdResult<PoolInfo> {
    let pool_info: InfoResponse =
        querier.query_wasm_smart(pool_contract_addr, &WasmSwapMsg::Info {})?;

    let token1_denom: AssetInfo = match pool_info.token1_denom {
        Denom::Native(denom) => AssetInfo::NativeToken { denom },
        Denom::Cw20(contract_addr) => AssetInfo::Cw20Token { contract_addr },
    };

    let token2_denom: AssetInfo = match pool_info.token2_denom {
        Denom::Native(denom) => AssetInfo::NativeToken { denom },
        Denom::Cw20(contract_addr) => AssetInfo::Cw20Token { contract_addr },
    };

    let res = PoolInfo {
        token1_reserve: pool_info.token1_reserve,
        token1_denom,
        token2_reserve: pool_info.token2_reserve,
        token2_denom,
        lp_token_address: pool_info.lp_token_address,
        lp_token_supply: pool_info.lp_token_supply,
    };
    Ok(res)
}

pub fn query_entire_system_coll(
    querier: &QuerierWrapper,
    active_pool_addr: Addr,
    default_pool_addr: Addr,
) -> StdResult<Uint128> {
    let active_coll: Uint128 =
        querier.query_wasm_smart(active_pool_addr, &ActivePoolQueryMsg::GetJUNO {})?;
    let liquidated_coll: Uint128 =
        querier.query_wasm_smart(default_pool_addr, &DefaultPoolQueryMsg::GetJUNO {})?;
    let total = active_coll
        .checked_add(liquidated_coll)
        .map_err(StdError::overflow)?;

    Ok(total)
}

pub fn query_entire_system_debt(
    querier: &QuerierWrapper,
    active_pool_addr: Addr,
    default_pool_addr: Addr,
) -> StdResult<Uint128> {
    let active_debt: Uint128 =
        querier.query_wasm_smart(active_pool_addr, &ActivePoolQueryMsg::GetULTRADebt {})?;
    let liquidated_debt: Uint128 =
        querier.query_wasm_smart(default_pool_addr, &DefaultPoolQueryMsg::GetULTRADebt {})?;
    let total = active_debt
        .checked_add(liquidated_debt)
        .map_err(StdError::overflow)?;

    Ok(total)
}

pub fn get_tcr(
    querier: &QuerierWrapper,
    price: Decimal256,
    active_pool_addr: Addr,
    default_pool_addr: Addr,
) -> StdResult<Decimal256> {
    let entire_system_coll =
        query_entire_system_debt(querier, active_pool_addr.clone(), default_pool_addr.clone())
            .unwrap();
    let entire_system_debt =
        query_entire_system_coll(querier, active_pool_addr, default_pool_addr).unwrap();
    let tcr = ultra_math::compute_cr(entire_system_coll, entire_system_debt, price).unwrap();
    Ok(tcr)
}

pub fn check_recovery_mode(
    querier: &QuerierWrapper,
    price: Decimal256,
    active_pool_addr: Addr,
    default_pool_addr: Addr,
) -> StdResult<bool> {
    let tcr = get_tcr(querier, price, active_pool_addr, default_pool_addr)?;
    Ok(tcr < CCR)
}

pub fn fetch_price(querier: &QuerierWrapper, price_feed_addr: Addr) -> StdResult<Decimal256> {
    let price: price_feed::GetPriceResponse =
        querier.query_wasm_smart(price_feed_addr, &price_feed::QueryMsg::GetPrice {})?;
    Ok(price.price)
}

pub fn query_borrowing_fee(
    querier: &QuerierWrapper,
    trove_manager_addr: Addr,
    stable_debt: Uint256,
) -> StdResult<Uint256> {
    let trove_manager::GetBorrowingFeeResponse { fee } = querier.query_wasm_smart(
        trove_manager_addr,
        &trove_manager::QueryMsg::GetBorrowingFee {
            ultra_debt: stable_debt,
        },
    )?;
    Ok(fee)
}

// function _requireUserAcceptsFee(uint _fee, uint _amount, uint _maxFeePercentage) internal pure {
//     uint feePercentage = _fee.mul(DECIMAL_PRECISION).div(_amount);
//     require(feePercentage <= _maxFeePercentage, "Fee exceeded provided maximum");
// }
// liquity source: https://github.com/liquity/dev/blob/7e5c38eff92c7de7b366ec791fd86abc2012952c/packages/contracts/contracts/Dependencies/LiquityBase.sol#L89-L93
pub fn require_user_accepts_fee(
    _querier: &QuerierWrapper,
    fee: Uint256,
    amount: Uint256,
    max_fee_percentage: Decimal256,
) -> StdResult<()> {
    let fee_with_precision = fee
        .checked_mul(DECIMAL_PRECISION.into())
        .map_err(StdError::overflow)?;
    let fee_percentage = Decimal256::from_ratio(fee_with_precision, amount);

    if fee_percentage > max_fee_percentage {
        return Err(StdError::generic_err("Fee exceeded provided maximum"));
    }

    Ok(())
}

// function _requireAtLeastMinNetDebt(uint _netDebt) internal pure {
//     require (_netDebt >= MIN_NET_DEBT, "BorrowerOps: Trove's net debt must be greater than minimum");
// }
// liquity source: https://github.com/liquity/dev/blob/e76b000e9558640e9479b8080786a9fbc47ed570/packages/contracts/contracts/BorrowerOperations.sol#L551-L553
pub fn require_at_least_min_net_debt(net_debt: Uint256) -> StdResult<()> {
    if net_debt < MIN_NET_DEBT.into() {
        return Err(StdError::generic_err(
            "BorrowerOps: Trove's net debt must be greater than minimum",
        ));
    }

    Ok(())
}

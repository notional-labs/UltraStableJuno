use crate::error::ContractError;
use crate::state::SUDO_PARAMS;
use cosmwasm_std::{entry_point, Addr, DepsMut, Env, Response};
use ultra_base::default_pool::SudoMsg;

pub struct ParamInfo {
    name: Option<String>,
    owner: Option<Addr>,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::UpdateParams { name, owner } => {
            sudo_update_params(deps, env, ParamInfo { name, owner })
        }
    }
}

/// Only governance can update contract params
pub fn sudo_update_params(
    deps: DepsMut,
    _env: Env,
    param_info: ParamInfo,
) -> Result<Response, ContractError> {
    let ParamInfo { name, owner } = param_info;

    let mut params = SUDO_PARAMS.load(deps.storage)?;

    params.name = name.unwrap_or(params.name);
    params.owner = owner.unwrap_or(params.owner);

    SUDO_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new().add_attribute("action", "update_params"))
}

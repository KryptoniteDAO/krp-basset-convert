use cosmwasm_std::{Binary, DepsMut, Env, MessageInfo, Response, Uint128};


use cw20_legacy::allowances::{
    execute_burn_from as cw20_burn_from, execute_send_from as cw20_send_from,
    execute_transfer_from as cw20_transfer_from,
};
use cw20_legacy::contract::{
    execute_burn as cw20_burn, execute_mint as cw20_mint, execute_send as cw20_send,
    execute_transfer as cw20_transfer,
};
use cw20_legacy::ContractError;

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let res: Response = cw20_transfer(deps, env, info, recipient, amount)?;
    Ok(Response::new().add_attributes(res.attributes))
}

pub fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let res: Response = cw20_burn(deps, env, info, amount)?;

    Ok(Response::new().add_attributes(res.attributes))
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let res: Response = cw20_mint(deps, env, info, recipient.clone(), amount)?;
    Ok(Response::new().add_attributes(res.attributes))
}

pub fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {

    let res: Response = cw20_send(deps, env, info, contract.clone(), amount, msg)?;

    Ok(Response::new().add_attributes(res.attributes).add_submessages(res.messages))
}

pub fn execute_transfer_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let res: Response = cw20_transfer_from(deps, env, info, owner, recipient.clone(), amount)?;

    Ok(Response::new().add_attributes(res.attributes))
}

pub fn execute_burn_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let res: Response = cw20_burn_from(deps, env, info, owner, amount)?;

    Ok(Response::new().add_attributes(res.attributes))
}

pub fn execute_send_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let res: Response = cw20_send_from(deps, env, info, owner, contract.clone(), amount, msg)?;
    Ok(Response::new().add_attributes(res.attributes).add_submessages(res.messages))
}

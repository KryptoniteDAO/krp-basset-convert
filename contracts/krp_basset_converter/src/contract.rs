#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::state::{read_config, store_config, Config};

use basset::converter::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Uint128, WasmMsg,
};

use crate::math::{convert_to_basset_decimals, convert_to_denom_decimals};
use crate::querier::query_decimals;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // cannot register the token at the inistantiation
    // because for the basset token contract, converter needs to be minter.
    let conf = Config {
        owner: deps.api.addr_canonicalize(&msg.owner)?,
        basset_token_address: None,
        native_denom: None,
    };

    store_config(deps.storage).save(&conf)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::RegisterTokens {
            basset_token_address,
            native_denom,
        } => register_tokens(deps, info, basset_token_address, native_denom),
        ExecuteMsg::ConvertNativeToBasset {} => execute_convert_to_basset(deps, env, info),
    }
}

/// CW20 token receive handler.
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let contract_addr = info.sender.clone();
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::ConvertBassetToNative {}) => {
            // only basset beth token contract can execute this message
            let conf = read_config(deps.storage)?;
            if deps.api.addr_canonicalize(contract_addr.as_str())?
                != conf.basset_token_address.unwrap()
            {
                return Err(StdError::generic_err("unauthorized"));
            }
            execute_convert_to_native(deps, env, info, cw20_msg.amount, cw20_msg.sender)
        }
        Err(err) => Err(err),
    }
}

pub fn register_tokens(
    deps: DepsMut,
    info: MessageInfo,
    basset_token_address: String,
    native_denom: String,
) -> StdResult<Response> {
    let mut config = read_config(deps.storage)?;

    if config.owner != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    // if the token contract is  already register we cannot change the address
    if config.basset_token_address.is_none() {
        config.basset_token_address = Some(deps.api.addr_canonicalize(&basset_token_address)?);
    }

    // if the token contract is  already register we cannot change the address
    if config.native_denom.is_none() {
        config.native_denom = Some(native_denom);
    }

    store_config(deps.storage).save(&config)?;

    Ok(Response::new().add_attributes(vec![("action", "register_token_contracts")]))
}

pub(crate) fn execute_convert_to_basset(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    if config.native_denom.is_none() || config.native_denom.is_none() {
        return Err(StdError::generic_err(
            "native denom must be registered first",
        ));
    }
    let coin_denom = config.native_denom.unwrap();

    let coin = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to deposit", coin_denom));
        });

    let denom_decimals = 6u8;

    let basset_decimals = query_decimals(
        deps.as_ref(),
        deps.api
            .addr_humanize(config.basset_token_address.as_ref().unwrap())
            .unwrap(),
    )?;

    // should convert to basset decimals
    let mint_amount =
        convert_to_basset_decimals(coin.unwrap().amount, basset_decimals, denom_decimals)?;
        
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps
                .api
                .addr_humanize(&config.basset_token_address.unwrap())?
                .to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount: mint_amount,
            })?,
            funds: vec![],
        }))
        .add_attributes(vec![
            ("action", "convert-to-basset"),
            ("recipient", &info.sender.to_string()),
            ("minted_amount", &mint_amount.to_string()),
        ]))
}

pub(crate) fn execute_convert_to_native(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    amount: Uint128,
    sender: String,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;
    if config.basset_token_address.is_none() || config.native_denom.is_none() {
        return Err(StdError::generic_err(
            "native or basset token must be registered first",
        ));
    }

    let denom_decimals = 6u8;

    let basset_decimals = query_decimals(
        deps.as_ref(),
        deps.api
            .addr_humanize(config.basset_token_address.as_ref().unwrap())
            .unwrap(),
    )?;

    // should convert to native decimals
    let return_amount = convert_to_denom_decimals(amount, basset_decimals, denom_decimals)?;

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.clone(),
                amount: vec![Coin {
                    amount: return_amount,
                    denom: config.native_denom.unwrap(),
                }],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps
                    .api
                    .addr_humanize(&config.basset_token_address.unwrap())?
                    .to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![
            ("action", "convert-to-native"),
            ("recipient", &sender),
            ("return_amount", &return_amount.to_string()),
            ("burn_amount", &amount.to_string()),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    let basset_token = if config.basset_token_address.is_some() {
        Some(
            deps.api
                .addr_humanize(&config.basset_token_address.unwrap())?
                .to_string(),
        )
    } else {
        None
    };
    let native_denom = if config.native_denom.is_some() {
        Some(config.native_denom.unwrap())
    } else {
        None
    };
    Ok(ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        basset_token_address: basset_token,
        native_denom,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

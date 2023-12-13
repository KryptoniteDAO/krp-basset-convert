#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::state::{
    read_config, read_new_owner, store_config, store_new_owner, Config, NewOwnerAddr,
};

use basset::converter::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, NewOwnerResponse, QueryMsg,
};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
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
        denom_decimals: None,
    };

    store_config(deps.storage).save(&conf)?;

    store_new_owner(
        deps.storage,
        &NewOwnerAddr {
            new_owner_addr: conf.owner.clone(),
        },
    )?;



    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::RegisterTokens {
            basset_token_address,
            native_denom,
            denom_decimals,
        } => register_tokens(
            deps,
            info,
            basset_token_address,
            native_denom,
            denom_decimals,
        ),
        ExecuteMsg::ConvertNativeToBasset {} => execute_convert_to_basset(deps, env, info),
        ExecuteMsg::SetOwner { new_owner_addr } => {
            let api = deps.api;
            set_new_owner(deps, info, api.addr_validate(&new_owner_addr)?)
        }
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info),
    }
}

pub fn set_new_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner_addr: Addr,
) -> StdResult<Response> {
    let config = read_config(deps.as_ref().storage)?;
    let mut new_owner = read_new_owner(deps.as_ref().storage)?;
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    if sender_raw != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }
    new_owner.new_owner_addr = deps.api.addr_canonicalize(&new_owner_addr.to_string())?;
    store_new_owner(deps.storage, &new_owner)?;

    Ok(Response::default())
}

pub fn accept_ownership(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let new_owner = read_new_owner(deps.as_ref().storage)?;
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    let mut config = read_config(deps.as_ref().storage)?;
    if sender_raw != new_owner.new_owner_addr {
        return Err(StdError::generic_err("unauthorized"));
    }

    config.owner = new_owner.new_owner_addr;
    store_config(deps.storage).save(&config)?;

    Ok(Response::default())
}

/// CW20 token receive handler.
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let contract_addr = info.sender.clone();
    match from_json(&cw20_msg.msg) {
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
    denom_decimals: u8,
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

    if config.denom_decimals.is_none() {
        config.denom_decimals = Some(denom_decimals);
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

    if info.funds.len() != 1 {
        return Err(StdError::generic_err(
            "The execute_convert_to_basset function only receives one registered native denom.",
        ));
    }

    let coin = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to deposit", coin_denom));
        });

    let basset_decimals = query_decimals(
        deps.as_ref(),
        deps.api
            .addr_humanize(config.basset_token_address.as_ref().unwrap())
            .unwrap(),
    )?;

    // should convert to basset decimals
    let mint_amount = convert_to_basset_decimals(
        coin.unwrap().amount,
        basset_decimals,
        config.denom_decimals.unwrap(),
    )?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps
                .api
                .addr_humanize(&config.basset_token_address.unwrap())?
                .to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
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

    let basset_decimals = query_decimals(
        deps.as_ref(),
        deps.api
            .addr_humanize(config.basset_token_address.as_ref().unwrap())
            .unwrap(),
    )?;

    // should convert to native decimals
    let return_amount =
        convert_to_denom_decimals(amount, basset_decimals, config.denom_decimals.unwrap())?;

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
                msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount })?,
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
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::NewOwner {} => to_json_binary(&query_new_owner(deps)?),
    }
}

pub fn query_new_owner(deps: Deps) -> StdResult<NewOwnerResponse> {
    let new_owner = read_new_owner(deps.storage)?;
    Ok(NewOwnerResponse {
        new_owner: deps
            .api
            .addr_humanize(&new_owner.new_owner_addr)?
            .to_string(),
    })
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
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

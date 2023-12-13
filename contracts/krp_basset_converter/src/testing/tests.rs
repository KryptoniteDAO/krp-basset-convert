use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    from_json, to_json_binary, Attribute, BankMsg, Coin, CosmosMsg, StdError, SubMsg, Uint128,
    WasmMsg,
};

use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies;
use basset::converter::Cw20HookMsg::ConvertBassetToNative;
use basset::converter::ExecuteMsg::{self, Receive, RegisterTokens};
use basset::converter::{ConfigResponse, InstantiateMsg, QueryMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

const MOCK_OWNER_ADDR: &str = "owner0000";
const MOCK_BASSET_TOKEN_CONTRACT_ADDR: &str = "cw20_token0000";
const MOCK_NATIVE_CONTRACT_ADDR: &str = "native_token0000";

fn default_init() -> InstantiateMsg {
    InstantiateMsg {
        owner: MOCK_OWNER_ADDR.to_string(),
    }
}

#[test]
fn proper_init() {
    let mut deps = mock_dependencies(&[]);
    let init_msg = default_init();

    let info = mock_info("addr0000", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_json(&res).unwrap();
    assert_eq!(
        config_response,
        ConfigResponse {
            owner: MOCK_OWNER_ADDR.to_string(),
            native_denom: None,
            basset_token_address: None,
        }
    );
}

#[test]
fn proper_convert_to_basset() {
    let mut deps = mock_dependencies(&[]);
    let init_msg = default_init();

    let sender = "addr0000";
    let info = mock_info(sender, &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let update_config = RegisterTokens {
        basset_token_address: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
        native_denom: MOCK_NATIVE_CONTRACT_ADDR.to_string(),
        denom_decimals: 8,
    };

    // set basset and native decimals
    deps.querier.set_decimals(6, 8);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_OWNER_ADDR, &[]),
        update_config,
    )
    .unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute::new("action", "register_token_contracts")
    );

    let msg = ExecuteMsg::ConvertNativeToBasset {};
    // unauthorized request
    // Native conversion of basset does not require permission, this test case does not require it
    // let invalid_info = mock_info("invalid", &[Coin::new(100000000u128, MOCK_NATIVE_CONTRACT_ADDR)]);
    // let error_res =
    //     execute(deps.as_mut(), mock_env(), invalid_info, msg.clone()).unwrap_err();
    // assert_eq!(error_res, StdError::generic_err("unauthorized"));

    // successful request
    let native_info = mock_info(
        sender,
        &[Coin::new(100000000u128, MOCK_NATIVE_CONTRACT_ADDR)],
    );
    let res = execute(deps.as_mut(), mock_env(), native_info, msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: sender.to_string(),
                // 100000000 / 10^2 = 1000000
                amount: Uint128::new(1000000)
            })
            .unwrap(),
            funds: vec![]
        }))
    );

    //cannot convert less than 100 micro native
    // let receive_msg = Receive(Cw20ReceiveMsg {
    //     sender: sender.to_string(),
    //     amount: Uint128::new(1),
    //     msg: to_json_binary(&ConvertBassetToNative {}).unwrap(),
    // });

    // unauthorized request
    // let invalid_info = mock_info("invalid", &[Coin::new(1u128, MOCK_NATIVE_CONTRACT_ADDR)]);
    // let error_res =
    //     execute(deps.as_mut(), mock_env(), invalid_info, msg.clone()).unwrap_err();
    // assert_eq!(error_res, StdError::generic_err("unauthorized"));

    // successful request
    let native_info = mock_info(sender, &[Coin::new(1u128, MOCK_NATIVE_CONTRACT_ADDR)]);
    let res = execute(deps.as_mut(), mock_env(), native_info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err(
            "cannot convert; conversion is only possible for amounts greater than 100 native token"
        )
    );
}

#[test]
fn proper_convert_to_native() {
    let mut deps = mock_dependencies(&[]);
    let init_msg = default_init();

    let sender = "addr0000";
    let info = mock_info(sender, &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let update_config = RegisterTokens {
        basset_token_address: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
        native_denom: MOCK_NATIVE_CONTRACT_ADDR.to_string(),
        denom_decimals: 8,
    };

    // set basset and native decimals
    deps.querier.set_decimals(6, 8);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_OWNER_ADDR, &[]),
        update_config,
    )
    .unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute::new("action", "register_token_contracts")
    );

    let receive_msg = Receive(Cw20ReceiveMsg {
        sender: sender.to_string(),
        amount: Uint128::new(100000000),
        msg: to_json_binary(&ConvertBassetToNative {}).unwrap(),
    });

    // unauthorized request
    let invalid_info = mock_info("invalid", &[]);
    let error_res =
        execute(deps.as_mut(), mock_env(), invalid_info, receive_msg.clone()).unwrap_err();
    assert_eq!(error_res, StdError::generic_err("unauthorized"));

    // successful
    let basset_info = mock_info(MOCK_BASSET_TOKEN_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), basset_info, receive_msg).unwrap();
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![Coin {
                // 100000000 * 10^2 = 10000000000
                amount: Uint128::new(10000000000),
                denom: MOCK_NATIVE_CONTRACT_ADDR.to_string(),
            }],
        }))
    );
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::new(100000000)
            })
            .unwrap(),
            funds: vec![]
        }))
    );
}

#[test]
fn proper_convert_to_basset_with_more_decimals() {
    let mut deps = mock_dependencies(&[]);
    let init_msg = default_init();

    let sender = "addr0000";
    let info = mock_info(sender, &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let update_config = RegisterTokens {
        basset_token_address: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
        native_denom: MOCK_NATIVE_CONTRACT_ADDR.to_string(),
        denom_decimals: 8,
    };

    // set basset and native decimals
    deps.querier.set_decimals(10, 8);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_OWNER_ADDR, &[]),
        update_config,
    )
    .unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute::new("action", "register_token_contracts")
    );

    // let receive_msg = Receive(Cw20ReceiveMsg {
    //     sender: sender.to_string(),
    //     amount: Uint128::new(100000000),
    //     msg: to_json_binary(&ConvertBassetToNative {}).unwrap(),
    // });

    // // unauthorized request
    // let invalid_info = mock_info("invalid", &[]);
    // let error_res =
    //     execute(deps.as_mut(), mock_env(), invalid_info, receive_msg.clone()).unwrap_err();
    // assert_eq!(error_res, StdError::generic_err("unauthorized"));

    let msg = ExecuteMsg::ConvertNativeToBasset {};
    // successful request
    let native_info = mock_info(
        sender,
        &[Coin::new(100000000u128, MOCK_NATIVE_CONTRACT_ADDR)],
    );
    let res = execute(deps.as_mut(), mock_env(), native_info, msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: sender.to_string(),
                //basset decimals is bigger than native then we should multiply with 10^2
                // 100000000 * 10^2 = 10000000000
                amount: Uint128::new(10000000000)
            })
            .unwrap(),
            funds: vec![]
        }))
    );
}

#[test]
fn proper_convert_to_native_with_less_decimals() {
    let mut deps = mock_dependencies(&[]);
    let init_msg = default_init();

    let sender = "addr0000";
    let info = mock_info(sender, &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let update_config = RegisterTokens {
        basset_token_address: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
        native_denom: MOCK_NATIVE_CONTRACT_ADDR.to_string(),
        denom_decimals: 8,
    };

    // set basset and native decimals
    deps.querier.set_decimals(10, 8);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_OWNER_ADDR, &[]),
        update_config,
    )
    .unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute::new("action", "register_token_contracts")
    );

    let receive_msg = Receive(Cw20ReceiveMsg {
        sender: sender.to_string(),
        amount: Uint128::new(100000000),
        msg: to_json_binary(&ConvertBassetToNative {}).unwrap(),
    });

    // unauthorized request
    let invalid_info = mock_info("invalid", &[]);
    let error_res =
        execute(deps.as_mut(), mock_env(), invalid_info, receive_msg.clone()).unwrap_err();
    assert_eq!(error_res, StdError::generic_err("unauthorized"));

    // successful
    let basset_info = mock_info(MOCK_BASSET_TOKEN_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), basset_info, receive_msg).unwrap();
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![Coin {
                // basset decimals is bigger than native then we should divide with 10^2
                // 100000000 * 10^2 = 1000000
                amount: Uint128::new(1000000),
                denom: MOCK_NATIVE_CONTRACT_ADDR.to_string(),
            }],
        }))
    );
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::new(100000000)
            })
            .unwrap(),
            funds: vec![]
        }))
    );

    //cannot convert less than 100 micro native
    let receive_msg = Receive(Cw20ReceiveMsg {
        sender: sender.to_string(),
        amount: Uint128::new(1),
        msg: to_json_binary(&ConvertBassetToNative {}).unwrap(),
    });

    // successful request
    let native_info = mock_info(MOCK_BASSET_TOKEN_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), native_info, receive_msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err(
            "cannot convert; conversion is only possible for amounts greater than 100 basset token"
        )
    );
}

#[test]
fn proper_update_config() {
    let mut deps = mock_dependencies(&[]);
    let init_msg = default_init();

    let info = mock_info(MOCK_OWNER_ADDR, &[]);
    let invalid_info = mock_info("invalid", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let update_config = RegisterTokens {
        basset_token_address: MOCK_BASSET_TOKEN_CONTRACT_ADDR.to_string(),
        native_denom: MOCK_NATIVE_CONTRACT_ADDR.to_string(),
        denom_decimals: 8,
    };

    // unauthorized request
    let error_res = execute(
        deps.as_mut(),
        mock_env(),
        invalid_info,
        update_config.clone(),
    )
    .unwrap_err();
    assert_eq!(error_res, StdError::generic_err("unauthorized"));

    //successful one
    let res = execute(deps.as_mut(), mock_env(), info, update_config).unwrap();
    assert_eq!(
        res.attributes[0],
        Attribute::new("action", "register_token_contracts")
    );

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_json(&res).unwrap();
    assert_eq!(
        config_response,
        ConfigResponse {
            owner: MOCK_OWNER_ADDR.to_string(),
            basset_token_address: Some("cw20_token0000".to_string()),
            native_denom: Some("native_token0000".to_string()),
        }
    );
}

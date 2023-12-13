use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, WasmQuery, Empty,
};

use cw20::TokenInfoResponse;

pub const MOCK_CONTRACT_ADDR: &str = "cosmos2contract";

pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = String::from(MOCK_CONTRACT_ADDR);
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
        MockQuerier::new(&[(&contract_addr, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: Default::default(),
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    // first one is CW20 token decimals, the second one is native token decimals
    decimals: (u8, u8),
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg: _,
            }) => {
                if contract_addr == "native_token0000" {
                    SystemResult::Ok(ContractResult::from(to_json_binary(&TokenInfoResponse {
                        name: "native_token".to_string(),
                        symbol: "DENOM".to_string(),
                        decimals: self.decimals.1,
                        total_supply: Default::default(),
                    })))
                } else {
                    SystemResult::Ok(ContractResult::from(to_json_binary(&TokenInfoResponse {
                        name: "basset_token".to_string(),
                        symbol: "CW2O".to_string(),
                        decimals: self.decimals.0,
                        total_supply: Default::default(),
                    })))
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier {
            base,
            decimals: (6, 8),
        }
    }

    pub fn set_decimals(&mut self, basset_token_decimals: u8, native_decimals: u8) {
        self.decimals = (basset_token_decimals, native_decimals)
    }
}

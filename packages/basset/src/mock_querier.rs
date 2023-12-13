use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, AllBalanceResponse, Api, BalanceResponse, BankQuery, CanonicalAddr,
    Coin, ContractResult, Decimal, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError,
    SystemResult, Uint128, WasmQuery,
};
use std::collections::HashMap;

use cw20::TokenInfoResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::common::{QueryTaxMsg, QueryTaxWrapper, TaxCapResponse, TaxRateResponse};

pub const MOCK_CONTRACT_ADDR: &str = "cosmos2contract";

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
        MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]),
        MockApi::default(),
    );

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: Default::default(),
    }
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
}

pub struct WasmMockQuerier {
    base: MockQuerier<QueryTaxWrapper>,
    tax_querier: TaxQuerier,
    // first one is basset token decimals, the second one is native token decimals
    decimals: (u8, u8),
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<QueryTaxWrapper> = match from_json(bin_request) {
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
    pub fn handle_query(&self, request: &QueryRequest<QueryTaxWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(QueryTaxWrapper { query_data }) => match query_data {
                QueryTaxMsg::TaxRate {} => {
                    let res = TaxRateResponse {
                        rate: self.tax_querier.rate,
                    };
                    SystemResult::Ok(ContractResult::from(to_json_binary(&res)))
                }
                QueryTaxMsg::TaxCap { denom } => {
                    let cap = self
                        .tax_querier
                        .caps
                        .get(denom)
                        .copied()
                        .unwrap_or_default();
                    let res = TaxCapResponse { cap };
                    SystemResult::Ok(ContractResult::from(to_json_binary(&res)))
                }
            },
            QueryRequest::Bank(BankQuery::AllBalances { address }) => {
                if address == &String::from("reward") {
                    let mut coins: Vec<Coin> = vec![];
                    let luna = Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::new(1000u128),
                    };
                    coins.push(luna);
                    let krt = Coin {
                        denom: "ukrt".to_string(),
                        amount: Uint128::new(1000u128),
                    };
                    coins.push(krt);
                    let all_balances = AllBalanceResponse { amount: coins };
                    SystemResult::Ok(ContractResult::from(to_json_binary(&all_balances)))
                } else {
                    unimplemented!()
                }
            }
            QueryRequest::Bank(BankQuery::Balance { address, denom }) => {
                if address == &String::from("reward") && denom == "uusd" {
                    let bank_res = BalanceResponse {
                        amount: Coin {
                            amount: Uint128::new(2000u128),
                            denom: denom.to_string(),
                        },
                    };
                    SystemResult::Ok(ContractResult::from(to_json_binary(&bank_res)))
                } else {
                    unimplemented!()
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg: _,
            }) => {
                if contract_addr == "native_token0000" {
                    SystemResult::Ok(ContractResult::from(to_json_binary(&TokenInfoResponse {
                        name: "native_token".to_string(),
                        symbol: "WORM".to_string(),
                        decimals: self.decimals.1,
                        total_supply: Default::default(),
                    })))
                } else {
                    SystemResult::Ok(ContractResult::from(to_json_binary(&TokenInfoResponse {
                        name: "cw_token".to_string(),
                        symbol: "cw20".to_string(),
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
    pub fn new<A: Api>(base: MockQuerier<QueryTaxWrapper>, _api: A) -> Self {
        WasmMockQuerier {
            base,
            tax_querier: TaxQuerier::default(),
            decimals: (6, 8),
        }
    }

    // configure the tax mock querier
    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    pub fn set_decimals(&mut self, basset_decimals: u8, native_decimals: u8) {
        self.decimals = (basset_decimals, native_decimals)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub mint: Option<MinterData>,
    pub owner: CanonicalAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MinterData {
    pub minter: CanonicalAddr,
    /// cap is how many more tokens can be issued by the minter
    pub cap: Option<Uint128>,
}

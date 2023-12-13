use std::ops::Deref;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Coin, Deps, StdResult};

use crate::common::CustomQuerier;

pub fn query_tax_rate_and_cap(deps: Deps, denom: String) -> StdResult<(Decimal256, Uint256)> {
    let custom_querier = CustomQuerier::new(deps.querier.deref());
    let rate = custom_querier.query_tax_rate()?.rate;
    let cap = custom_querier.query_tax_cap(denom)?.cap;

    Ok((rate.into(), cap.into()))
}

pub fn query_tax_rate(deps: Deps) -> StdResult<Decimal256> {
    let custom_querier = CustomQuerier::new(deps.querier.deref());
    Ok(custom_querier.query_tax_rate()?.rate.into())
}

pub fn compute_tax(deps: Deps, coin: &Coin) -> StdResult<Uint256> {
    let custom_querier = CustomQuerier::new(deps.querier.deref());
    let tax_rate = Decimal256::from((custom_querier.query_tax_rate()?).rate);
    let tax_cap = Uint256::from((custom_querier.query_tax_cap(coin.denom.to_string())?).cap);
    let amount = Uint256::from(coin.amount);
    Ok(std::cmp::min(
        amount * Decimal256::one() - amount / (Decimal256::one() + tax_rate),
        tax_cap,
    ))
}

pub fn deduct_tax(deps: Deps, coin: Coin) -> StdResult<Coin> {
    let tax_amount = compute_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: (Uint256::from(coin.amount) - tax_amount).into(),
    })
}

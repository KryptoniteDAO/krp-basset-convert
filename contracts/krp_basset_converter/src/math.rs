use cosmwasm_std::{StdError, StdResult, Uint128};

pub(crate) fn convert_to_denom_decimals(
    amount: Uint128,
    basset_decimals: u8,
    denom_decimals: u8,
) -> StdResult<Uint128> {
    if basset_decimals > denom_decimals {
        let decimal_fraction =
            Uint128::new(10u128).saturating_pow((basset_decimals - denom_decimals) as u32);
        let result = amount.checked_div(decimal_fraction);
        if result.as_ref().unwrap().is_zero() {
            return Err(StdError::generic_err(format!(
                "cannot convert; conversion is only possible for amounts greater than {} basset token",
                decimal_fraction
            )));
        }
        Ok(result.unwrap())
    } else {
        let decimal_fraction =
            Uint128::new(10u128).saturating_pow((denom_decimals - basset_decimals) as u32);
        Ok(amount.checked_mul(decimal_fraction).unwrap())
    }
}

pub(crate) fn convert_to_basset_decimals(
    amount: Uint128,
    basset_decimals: u8,
    denom_decimals: u8,
) -> StdResult<Uint128> {
    if basset_decimals > denom_decimals {
        let decimal_fraction =
            Uint128::new(10u128).saturating_pow((basset_decimals - denom_decimals) as u32);

        Ok(amount.checked_mul(decimal_fraction).unwrap())
    } else {
        let decimal_fraction =
            Uint128::new(10u128).saturating_pow((denom_decimals - basset_decimals) as u32);
        let result = amount.checked_div(decimal_fraction);
        if result.as_ref().unwrap().is_zero() {
            return Err(StdError::generic_err(format!(
                "cannot convert; conversion is only possible for amounts greater than {} native token",
                decimal_fraction
            )));
        }
        Ok(result.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_denom_decimals() {
        let a = Uint128::new(100000000);
        let b = 4;
        let c = 6;
        let d = convert_to_denom_decimals(a, b, c).unwrap();
        assert_eq!(d, Uint128::new(10000000000));
    }

    #[test]
    fn test_convert_to_basset_decimals() {
        let a = Uint128::new(100000000);
        let b = 4;
        let c = 6;
        let d = convert_to_basset_decimals(a, b, c).unwrap();
        assert_eq!(d, Uint128::new(1000000));
    }
}

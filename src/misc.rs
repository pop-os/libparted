//! Implements some miscellanious functions from the libparted API. These aren't taken from
//! the libparted bindings as it's trivial to write them ourselves.

fn abs_mod(a: i64, b: i64) -> i64 {
    if a < 0 {
        a % b + b
    } else {
        a % b
    }
}

/// Rounds a number down to the closest number that is a multiple of the grain size.
pub fn round_down_to(sector: i64, grain_size: i64) -> i64 {
    sector - abs_mod(sector, grain_size)
}

/// Rounds a number up to the closest number that is a multiple of the grain size.
pub fn round_up_to(sector: i64, grain_size: i64) -> i64 {
    if sector % grain_size != 0 {
        round_down_to(sector, grain_size) + grain_size
    } else {
        sector
    }
}

/// Rounds a number to the closest number that is a multiple of the grain_size.
pub fn round_to_nearest(sector: i64, grain_size: i64) -> i64 {
    if sector % grain_size > grain_size / 2 {
        round_up_to(sector, grain_size)
    } else {
        round_down_to(sector, grain_size)
    }
}

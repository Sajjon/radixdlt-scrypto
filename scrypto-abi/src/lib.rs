#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!("Either feature `std` or `alloc` must be enabled for this crate.");
#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!("Feature `std` and `alloc` can't be enabled at the same time.");

mod blueprint_abi;
mod schema_type;

pub use blueprint_abi::*;
pub use schema_type::*;

mod schema;
mod schema_aggregator;
mod type_ref;
mod type_schema;
mod basic_impls;

pub mod v2 {
    use super::*;
    pub use schema::*;
    pub use schema_aggregator::*;
    pub use type_ref::*;
    pub use type_schema::*;
}

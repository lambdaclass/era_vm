use u256::U256;

use crate::heaps::Heaps;

pub mod sha256;

pub trait Precompile: std::fmt::Debug {
    fn execute_precompile(&mut self, abi_key: U256, heaps: &mut Heaps);
}

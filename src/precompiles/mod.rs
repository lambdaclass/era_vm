use crate::{eravm_error::EraVmError, heaps::Heaps};
use std::ptr;
use u256::U256;

pub mod ecrecover;
pub mod keccak256;
pub mod secp256r1_verify;
pub mod sha256;

pub trait Precompile: std::fmt::Debug {
    fn execute_precompile(&mut self, abi_key: U256, heaps: &mut Heaps) -> Result<(), EraVmError>;
}

pub struct PrecompileCallABI {
    pub input_memory_offset: u32,
    pub input_memory_length: u32,
    pub output_memory_offset: u32,
    pub _output_memory_length: u32,
    pub memory_page_to_read: u32,
    pub memory_page_to_write: u32,
    pub precompile_interpreted_data: u64,
}

impl PrecompileCallABI {
    pub const fn from_u256(raw_value: U256) -> Self {
        let raw = raw_value.0;
        let input_memory_offset = raw[0] as u32;
        let input_memory_length = (raw[0] >> 32) as u32;
        let output_memory_offset = raw[1] as u32;
        let _output_memory_length = (raw[1] >> 32) as u32;
        let memory_page_to_read = raw[2] as u32;
        let memory_page_to_write = (raw[2] >> 32) as u32;
        let precompile_interpreted_data = raw[3];

        Self {
            input_memory_offset,
            input_memory_length,
            output_memory_offset,
            _output_memory_length,
            memory_page_to_read,
            memory_page_to_write,
            precompile_interpreted_data,
        }
    }
}

pub fn precompile_abi_in_log(abi_key: U256) -> PrecompileCallABI {
    PrecompileCallABI::from_u256(abi_key)
}

struct Sha3State<T> {
    state: T,
}

struct BlockBuffer<const N: usize> {
    _buffer: [u8; N],
    _pos: u8,
}

struct CoreWrapper<T, const N: usize> {
    core: Sha3State<T>,
    _buffer: BlockBuffer<N>,
}

fn get_hasher_state<const N: usize, T, H>(hasher: H) -> T {
    // casts the hasher ptr to the CoreWrapper struct
    let raw_ptr = &hasher as *const _ as *const CoreWrapper<T, N>;
    // this is not unsafe since we are replicating the structure(thus same layout) of the original ptr
    // this a hack that allows us to access private fields
    // also ptr::read does not moved the value from memory
    unsafe { ptr::read(raw_ptr) }.core.state
}

fn get_hasher_state_136<T, H>(hasher: H) -> T {
    get_hasher_state::<136, T, H>(hasher)
}

fn get_hasher_state_64<T, H>(hasher: H) -> T {
    get_hasher_state::<64, T, H>(hasher)
}

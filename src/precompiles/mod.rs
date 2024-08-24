use crate::{eravm_error::EraVmError, heaps::Heaps};
use std::{mem::size_of, ptr};
use u256::U256;
use zkevm_opcode_defs::{sha2::Sha256VarCore, sha3::Keccak256Core};

pub mod ecrecover;
pub mod keccak256;
pub mod secp256r1_verify;
pub mod sha256;

pub trait Precompile: std::fmt::Debug {
    fn execute_precompile(&mut self, abi_key: U256, heaps: &mut Heaps)
        -> Result<usize, EraVmError>;
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

struct State<T> {
    state: T,
    round_count: usize,
}

// check at compile time that the state struct is of the same size as the hashers
const fn _assert_eq_size<T, K>() {
    let res = size_of::<T>() == size_of::<K>();
    if !res {
        panic!();
    }
}
const _: () = _assert_eq_size::<State<[u32; 8]>, Sha256VarCore>();
const _: () = _assert_eq_size::<State<[u64; 25]>, Keccak256Core>();

fn get_hasher_state<T, Core>(core: Core, round_count: usize) -> Result<T, EraVmError> {
    // casts the hasher ptr to the CoreWrapper struct
    let raw_ptr = &core as *const _ as *const State<T>;
    // this is not unsafe since we are replicating the structure(thus same layout) of the original ptr
    // this a hack that allows us to access private fields
    // also ptr::read does not moved the value from memory
    let core_r = unsafe { ptr::read(raw_ptr) };

    // sanity check to make sure the cast was ok
    if core_r.round_count == round_count {
        Ok(core_r.state)
    } else {
        Err(EraVmError::OutOfGas)
    }
}

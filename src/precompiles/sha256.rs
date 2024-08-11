use super::{precompile_abi_in_log, Precompile};
use crate::{eravm_error::EraVmError, heaps::Heaps};
use std::ptr;
use u256::U256;
pub use zkevm_opcode_defs::sha2::Digest;
pub use zkevm_opcode_defs::sha2::Sha256;

pub const MEMORY_READS_PER_CYCLE: usize = 2;

struct Sha256VarCore {
    state: [u32; 8],
}

struct CoreWrapper {
    core: Sha256VarCore,
}

fn get_hasher_state(hasher: Sha256) -> [u32; 8] {
    // casts the hasher ptr to the CoreWrapper struct
    let raw_ptr = &hasher as *const _ as *const CoreWrapper;
    // this is not unsafe since we are replicating the structure of the original ptr
    // this allows us to access private fields
    unsafe { ptr::read(raw_ptr) }.core.state
}

fn get_hash_from_state(state: [u32; 8]) -> [u8; 32] {
    let mut hash_as_bytes32 = [0; 32];
    for (chunk, state_word) in hash_as_bytes32.chunks_mut(4).zip(state.into_iter()) {
        chunk.copy_from_slice(&state_word.to_be_bytes());
    }
    hash_as_bytes32
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sha256Precompile;

impl Precompile for Sha256Precompile {
    fn execute_precompile(&mut self, abi_key: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
        let params = precompile_abi_in_log(abi_key);
        let num_rounds = params.precompile_interpreted_data as usize;
        let mut read_addr = params.input_memory_offset;
        let write_addr = params.output_memory_offset * 32;

        let mut hasher = Sha256::new();
        for _ in 0..num_rounds {
            let mut block = [0u8; 64];

            for query_index in 0..MEMORY_READS_PER_CYCLE {
                let (data, _) = heaps
                    .try_get_mut(params.memory_page_to_read)?
                    .expanded_read(read_addr * 32);
                read_addr += 1;
                data.to_big_endian(&mut block[(query_index * 32)..(query_index * 32 + 32)]);
            }

            hasher.update(&block);
        }
        let state = get_hasher_state(hasher);
        let hash = U256::from_big_endian(&get_hash_from_state(state));
        heaps
            .try_get_mut(params.memory_page_to_write)?
            .store(write_addr, hash);

        Ok(())
    }
}

pub fn sha256_rounds_function(abi_key: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
    Sha256Precompile.execute_precompile(abi_key, heaps)
}

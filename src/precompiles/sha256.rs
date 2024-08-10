use super::{precompile_abi_in_log, Precompile};
use crate::{eravm_error::EraVmError, heaps::Heaps};
use crypto_common::hazmat::SerializableState;
use sha2::{Digest, Sha256};
use u256::U256;

pub const MEMORY_READS_PER_CYCLE: usize = 2;

fn get_state_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut result = [0u8; 32];

    for i in 0..8 {
        result[i * 4] = bytes[i * 4 + 3];
        result[i * 4 + 1] = bytes[i * 4 + 2];
        result[i * 4 + 2] = bytes[i * 4 + 1];
        result[i * 4 + 3] = bytes[i * 4];
    }

    result
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
        for round in 0..num_rounds {
            let mut block = [0u8; 64];

            for query_index in 0..MEMORY_READS_PER_CYCLE {
                let (data, _) = heaps
                    .try_get_mut(params.memory_page_to_read)?
                    .expanded_read(read_addr * 32);
                read_addr += 1;
                data.to_big_endian(&mut block[(query_index * 32)..(query_index * 32 + 32)]);
            }

            hasher.update(block);

            let is_last = round == num_rounds - 1;

            if is_last {
                let raw_bytes = hasher.clone().serialize();
                let state_bytes = get_state_bytes(&raw_bytes[0..32]); // state is in first 32 bytes
                let as_u256 = U256::from_big_endian(&state_bytes);

                heaps
                    .try_get_mut(params.memory_page_to_write)?
                    .store(write_addr, as_u256);
            }
        }
        Ok(())
    }
}

pub fn sha256_rounds_function(abi_key: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
    Sha256Precompile.execute_precompile(abi_key, heaps)
}

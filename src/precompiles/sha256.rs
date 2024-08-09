use super::{precompile_abi_in_log, MemoryLocation, MemoryQuery, Precompile};
use crate::{eravm_error::EraVmError, heaps::Heaps};
use crypto_common::hazmat::SerializableState;
use sha2::{Digest, Sha256};
use u256::U256;

pub const MEMORY_READS_PER_CYCLE: usize = 2;
pub const MEMORY_WRITES_PER_CYCLE: usize = 1;

fn get_state_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut result = [0u8; 32];

    // grab in chunks of 4 bytes and reverse them
    for i in 0..8 {
        result[i * 4] = bytes[i * 4 + 3];
        result[i * 4 + 1] = bytes[i * 4 + 2];
        result[i * 4 + 2] = bytes[i * 4 + 1];
        result[i * 4 + 3] = bytes[i * 4];
    }

    result
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sha256RoundWitness {
    pub new_request: Option<U256>,
    pub reads: [MemoryQuery; MEMORY_READS_PER_CYCLE],
    pub writes: Option<[MemoryQuery; MEMORY_WRITES_PER_CYCLE]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sha256Precompile;

impl Precompile for Sha256Precompile {
    fn execute_precompile(&mut self, abi_key: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
        let params = precompile_abi_in_log(abi_key);
        let num_rounds = params.precompile_interpreted_data as usize;
        let source_memory_page = params.memory_page_to_read;
        let destination_memory_page = params.memory_page_to_write;
        let mut current_read_offset = params.input_memory_offset;
        let write_offset = params.output_memory_offset;

        let mut hasher = Sha256::new();
        for round in 0..num_rounds {
            let mut block = [0u8; 64];

            let mut reads = [MemoryQuery::empty(); MEMORY_READS_PER_CYCLE];
            for query_index in 0..MEMORY_READS_PER_CYCLE {
                let query = MemoryQuery {
                    location: MemoryLocation {
                        page: source_memory_page,
                        index: current_read_offset,
                    },
                    value: U256::zero(),
                    value_is_pointer: false,
                    rw_flag: false,
                };

                let query = heaps.execute_partial_query(query)?;
                current_read_offset += 1;

                reads[query_index] = query;
                let data = query.value;
                data.to_big_endian(&mut block[(query_index * 32)..(query_index * 32 + 32)]);
            }

            hasher.update(block);

            let is_last = round == num_rounds - 1;

            let mut round_witness = Sha256RoundWitness {
                new_request: None,
                reads,
                writes: None,
            };

            if round == 0 {
                round_witness.new_request = Some(abi_key);
            }

            if is_last {
                let raw_bytes = hasher.clone().serialize();
                let state_bytes = get_state_bytes(&raw_bytes[0..32]); // state is in first 32 bytes
                let as_u256 = U256::from_big_endian(&state_bytes);

                let write_location = MemoryLocation {
                    page: destination_memory_page,
                    index: write_offset,
                };

                let result_query = MemoryQuery {
                    location: write_location,
                    value: as_u256,
                    value_is_pointer: false,
                    rw_flag: true,
                };

                let result_query = heaps.execute_partial_query(result_query)?;
                round_witness.writes = Some([result_query]);
            }
        }
        Ok(())
    }
}

pub fn sha256_rounds_function(abi_key: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
    Sha256Precompile.execute_precompile(abi_key, heaps)
}

use std::hash::Hasher;

use super::{precompile_abi_in_log, MemoryLocation, MemoryQuery, Precompile};
use crate::heaps::Heaps;
use rs_sha256::{Sha256Hasher, Sha256State};
use zkevm_opcode_defs::ethereum_types::U256;

pub const MEMORY_READS_PER_CYCLE: usize = 2;
pub const MEMORY_WRITES_PER_CYCLE: usize = 1;

fn get_inner_state_bytes(inner_state: Sha256State) -> [u8; 32] {
    let mut u32_bytes = [0u32; 8];
    u32_bytes[0] = inner_state.0.into();
    u32_bytes[1] = inner_state.1.into();
    u32_bytes[2] = inner_state.2.into();
    u32_bytes[3] = inner_state.3.into();
    u32_bytes[4] = inner_state.4.into();
    u32_bytes[5] = inner_state.5.into();
    u32_bytes[6] = inner_state.6.into();
    u32_bytes[7] = inner_state.7.into();

    let mut result = [0u8; 32];
    for i in 0..8 {
        result[i * 4] = (u32_bytes[i] >> 24) as u8;
        result[i * 4 + 1] = (u32_bytes[i] >> 16) as u8;
        result[i * 4 + 2] = (u32_bytes[i] >> 8) as u8;
        result[i * 4 + 3] = u32_bytes[i] as u8;
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
    fn execute_precompile(&mut self, abi_key: U256, heaps: &mut Heaps) {
        let params = precompile_abi_in_log(abi_key);

        let num_rounds = params.precompile_interpreted_data as usize;
        let source_memory_page = params.memory_page_to_read;
        let destination_memory_page = params.memory_page_to_write;
        let mut current_read_offset = params.input_memory_offset;
        let write_offset = params.output_memory_offset;

        let mut hasher = Sha256Hasher::default();
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

                let query = heaps.new_execute_partial_query(query);
                current_read_offset += 1;

                reads[query_index] = query;
                let data = query.value;
                data.to_big_endian(&mut block[(query_index * 32)..(query_index * 32 + 32)]);
            }

            // run round function
            hasher.write(&block);

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
                let inner_state: Sha256State = hasher.clone().into();
                let bytes = get_inner_state_bytes(inner_state);
                let as_u256 = U256::from_big_endian(&bytes);

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

                // TODO: wrap function in result query
                let result_query = heaps.new_execute_partial_query(result_query);
                round_witness.writes = Some([result_query]);
            }
        }
    }
}

pub fn sha256_rounds_function(abi_key: U256, heaps: &mut Heaps) {
    let mut processor = Sha256Precompile;
    processor.execute_precompile(abi_key, heaps);
}

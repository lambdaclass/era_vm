use super::{precompile_abi_in_log, MemoryLocation, MemoryQuery, Precompile};
use crate::heaps::Heaps;
use zkevm_opcode_defs::ethereum_types::U256;
use zkevm_opcode_defs::sha2::Digest;
use zkevm_opcode_defs::sha3::Keccak256;

pub const KECCAK_RATE_BYTES: usize = 136;
pub const MEMORY_READS_PER_CYCLE: usize = 6;
pub const KECCAK_PRECOMPILE_BUFFER_SIZE: usize = MEMORY_READS_PER_CYCLE * 32;
pub const MEMORY_WRITES_PER_CYCLE: usize = 1;

pub struct ByteBuffer<const BUFFER_SIZE: usize> {
    pub bytes: [u8; BUFFER_SIZE],
    pub filled: usize,
}

impl<const BUFFER_SIZE: usize> ByteBuffer<BUFFER_SIZE> {
    pub fn can_fill_bytes(&self, num_bytes: usize) -> bool {
        self.filled + num_bytes <= BUFFER_SIZE
    }

    pub fn fill_with_bytes<const N: usize>(
        &mut self,
        input: &[u8; N],
        offset: usize,
        meaningful_bytes: usize,
    ) {
        assert!(self.filled + meaningful_bytes <= BUFFER_SIZE);
        self.bytes[self.filled..(self.filled + meaningful_bytes)]
            .copy_from_slice(&input[offset..(offset + meaningful_bytes)]);
        self.filled += meaningful_bytes;
    }

    pub fn consume<const N: usize>(&mut self) -> [u8; N] {
        assert!(N <= BUFFER_SIZE);
        let mut result = [0u8; N];
        result.copy_from_slice(&self.bytes[..N]);
        if self.filled < N {
            self.filled = 0;
        } else {
            self.filled -= N;
        }
        let mut new_bytes = [0u8; BUFFER_SIZE];
        new_bytes[..(BUFFER_SIZE - N)].copy_from_slice(&self.bytes[N..]);
        self.bytes = new_bytes;

        result
    }
}

pub type Keccak256InnerState = [u64; 25];

struct Sha3State {
    state: [u64; 25],
    _round_count: usize,
}

struct BlockBuffer {
    _buffer: [u8; 136],
    _pos: u8,
}

struct CoreWrapper {
    core: Sha3State,
    _buffer: BlockBuffer,
}

pub fn transmute_state(reference_state: Keccak256) -> Keccak256InnerState {
    // we use a trick that size of both structures is the same, and even though we do not know a stable field layout,
    // we can replicate it
    let our_wrapper: CoreWrapper = unsafe { std::mem::transmute(reference_state) };

    our_wrapper.core.state
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Keccak256RoundWitness {
    pub new_request: Option<U256>,
    pub reads: Option<[MemoryQuery; MEMORY_READS_PER_CYCLE]>,
    pub writes: Option<[MemoryQuery; MEMORY_WRITES_PER_CYCLE]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Keccak256Precompile;

impl Precompile for Keccak256Precompile {
    fn execute_precompile(&mut self, abi_key: U256, heaps: &mut Heaps) {
        let mut full_round_padding = [0u8; KECCAK_RATE_BYTES];
        full_round_padding[0] = 0x01;
        full_round_padding[KECCAK_RATE_BYTES - 1] = 0x80;

        let params = precompile_abi_in_log(abi_key);

        let mut input_byte_offset = params.input_memory_offset as usize;
        let mut bytes_left = params.input_memory_length as usize;

        let mut num_rounds = (bytes_left + (KECCAK_RATE_BYTES - 1)) / KECCAK_RATE_BYTES;
        let padding_space = bytes_left % KECCAK_RATE_BYTES;
        let needs_extra_padding_round = padding_space == 0;
        if needs_extra_padding_round {
            num_rounds += 1;
        }

        let source_memory_page = params.memory_page_to_read;
        let destination_memory_page = params.memory_page_to_write;
        let write_offset = params.output_memory_offset;

        let mut input_buffer = ByteBuffer::<KECCAK_PRECOMPILE_BUFFER_SIZE> {
            bytes: [0u8; KECCAK_PRECOMPILE_BUFFER_SIZE],
            filled: 0,
        };

        let mut hasher: Keccak256 = Keccak256::default();

        for round in 0..num_rounds {
            let is_last = round == num_rounds - 1;
            let paddings_round = needs_extra_padding_round && is_last;

            let mut bytes32_buffer = [0u8; 32];
            for _idx in 0..MEMORY_READS_PER_CYCLE {
                let (memory_index, unalignment) = (input_byte_offset / 32, input_byte_offset % 32);
                let at_most_meaningful_bytes_in_query = 32 - unalignment;
                let meaningful_bytes_in_query = if bytes_left >= at_most_meaningful_bytes_in_query {
                    at_most_meaningful_bytes_in_query
                } else {
                    bytes_left
                };

                let enough_buffer_space = input_buffer.can_fill_bytes(meaningful_bytes_in_query);
                let nothing_to_read = meaningful_bytes_in_query == 0;
                let should_read =
                    nothing_to_read == false && paddings_round == false && enough_buffer_space;

                let bytes_to_fill = if should_read {
                    meaningful_bytes_in_query
                } else {
                    0
                };

                if should_read {
                    input_byte_offset += meaningful_bytes_in_query;
                    bytes_left -= meaningful_bytes_in_query;

                    let data_query = MemoryQuery {
                        location: MemoryLocation {
                            page: source_memory_page,
                            index: memory_index as u32,
                        },
                        value: U256::zero(),
                        value_is_pointer: false,
                        rw_flag: false,
                    };
                    let data_query = heaps.new_execute_partial_query(data_query);
                    let data = data_query.value;
                    data.to_big_endian(&mut bytes32_buffer[..]);
                }

                input_buffer.fill_with_bytes(&bytes32_buffer, unalignment, bytes_to_fill)
            }

            let mut block = input_buffer.consume::<KECCAK_RATE_BYTES>();
            // apply padding
            if paddings_round {
                block = full_round_padding;
            } else if is_last {
                if padding_space == KECCAK_RATE_BYTES - 1 {
                    block[KECCAK_RATE_BYTES - 1] = 0x81;
                } else {
                    block[padding_space] = 0x01;
                    block[KECCAK_RATE_BYTES - 1] = 0x80;
                }
            }
            // update the keccak internal state
            hasher.update(&block);

            if is_last {
                let state_inner = transmute_state(hasher.clone());

                // take hash and properly set endianess for the output word
                let mut hash_as_bytes32 = [0u8; 32];
                hash_as_bytes32[0..8].copy_from_slice(&state_inner[0].to_le_bytes());
                hash_as_bytes32[8..16].copy_from_slice(&state_inner[1].to_le_bytes());
                hash_as_bytes32[16..24].copy_from_slice(&state_inner[2].to_le_bytes());
                hash_as_bytes32[24..32].copy_from_slice(&state_inner[3].to_le_bytes());
                let as_u256 = U256::from_big_endian(&hash_as_bytes32);
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

                // TODO: wrap execute in Result and handle
                let _result_query = heaps.new_execute_partial_query(result_query);
            }
        }
    }
}

pub fn keccak256_rounds_function(abi_key: U256, heaps: &mut Heaps) {
    let mut processor = Keccak256Precompile;
    processor.execute_precompile(abi_key, heaps);
}

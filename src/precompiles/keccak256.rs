use super::get_hasher_state;
use super::{precompile_abi_in_log, Precompile};
use crate::{eravm_error::EraVmError, heaps::Heaps};
use u256::U256;
pub use zkevm_opcode_defs::sha2::Digest;
pub use zkevm_opcode_defs::sha3::Keccak256;

pub const KECCAK_RATE_BYTES: usize = 136;
pub const KECCAK_ROUND_COUNT: usize = 24;
pub const MEMORY_READS_PER_CYCLE: usize = 6;
pub const KECCAK_PRECOMPILE_BUFFER_SIZE: usize = MEMORY_READS_PER_CYCLE * 32;

pub struct ByteBuffer {
    pub bytes: [u8; KECCAK_PRECOMPILE_BUFFER_SIZE],
    pub filled: usize,
}

impl Default for ByteBuffer {
    fn default() -> Self {
        Self {
            bytes: [0u8; KECCAK_PRECOMPILE_BUFFER_SIZE],
            filled: 0,
        }
    }
}

impl ByteBuffer {
    pub fn can_fill_bytes(&self, num_bytes: usize) -> bool {
        self.filled + num_bytes <= KECCAK_PRECOMPILE_BUFFER_SIZE
    }

    pub fn fill_with_bytes<const N: usize>(
        &mut self,
        input: &[u8; N],
        offset: usize,
        meaningful_bytes: usize,
    ) {
        assert!(self.filled + meaningful_bytes <= KECCAK_PRECOMPILE_BUFFER_SIZE);
        self.bytes[self.filled..(self.filled + meaningful_bytes)]
            .copy_from_slice(&input[offset..(offset + meaningful_bytes)]);
        self.filled += meaningful_bytes;
    }

    pub fn consume<const N: usize>(&mut self) -> [u8; N] {
        assert!(N <= KECCAK_PRECOMPILE_BUFFER_SIZE);
        let mut result = [0u8; N];
        result.copy_from_slice(&self.bytes[..N]);
        if self.filled < N {
            self.filled = 0;
        } else {
            self.filled -= N;
        }
        let mut new_bytes = [0u8; KECCAK_PRECOMPILE_BUFFER_SIZE];
        new_bytes[..(KECCAK_PRECOMPILE_BUFFER_SIZE - N)].copy_from_slice(&self.bytes[N..]);
        self.bytes = new_bytes;

        result
    }
}

fn hash_as_bytes32(hash: [u64; 25]) -> [u8; 32] {
    let mut result = [0; 32];
    for i in 0..4 {
        result[i * 8..(i + 1) * 8].copy_from_slice(&hash[i].to_le_bytes());
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Keccak256Precompile;

impl Precompile for Keccak256Precompile {
    fn execute_precompile(
        &mut self,
        abi_key: U256,
        heaps: &mut Heaps,
    ) -> Result<usize, EraVmError> {
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

        let mut input_buffer = ByteBuffer::default();

        let mut hasher = Keccak256::new();
        let heap_to_read = heaps.try_get_mut(params.memory_page_to_read)?;

        for round in 0..num_rounds {
            let is_last = round == num_rounds - 1;
            let paddings_round = needs_extra_padding_round && is_last;

            let mut bytes32_buffer = [0u8; 32];

            for _idx in 0..MEMORY_READS_PER_CYCLE {
                let (read_addr, unalignment) = (input_byte_offset / 32, input_byte_offset % 32);
                let at_most_meaningful_bytes_in_query = 32 - unalignment;
                let meaningful_bytes_in_query = if bytes_left >= at_most_meaningful_bytes_in_query {
                    at_most_meaningful_bytes_in_query
                } else {
                    bytes_left
                };

                let enough_buffer_space = input_buffer.can_fill_bytes(meaningful_bytes_in_query);
                let nothing_to_read = meaningful_bytes_in_query == 0;
                let should_read = !nothing_to_read && !paddings_round && enough_buffer_space;

                let bytes_to_fill = if should_read {
                    meaningful_bytes_in_query
                } else {
                    0
                };

                if should_read {
                    let (data, _) = heap_to_read.expanded_read(read_addr as u32 * 32);
                    data.to_big_endian(&mut bytes32_buffer[..]);
                    input_byte_offset += meaningful_bytes_in_query;
                    bytes_left -= meaningful_bytes_in_query;
                }

                input_buffer.fill_with_bytes(&bytes32_buffer, unalignment, bytes_to_fill)
            }

            let mut block = input_buffer.consume();
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

            hasher.update(block);
        }
        let state: [u64; 25] = get_hasher_state(hasher.decompose().0, KECCAK_ROUND_COUNT)?;
        let hash = U256::from_big_endian(&hash_as_bytes32(state));
        heaps
            .try_get_mut(params.memory_page_to_write)?
            .store(params.output_memory_offset * 32, hash);

        Ok(num_rounds)
    }
}

pub fn keccak256_rounds_function(abi_key: U256, heaps: &mut Heaps) -> Result<usize, EraVmError> {
    Keccak256Precompile.execute_precompile(abi_key, heaps)
}

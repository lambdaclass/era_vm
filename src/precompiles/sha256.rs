use super::Precompile;
use crate::heaps::{precompile_abi_in_log, Heaps, MemoryLocation, MemoryQuery};
use zkevm_opcode_defs::ethereum_types::U256;

pub const MEMORY_READS_PER_CYCLE: usize = 2;
pub const MEMORY_WRITES_PER_CYCLE: usize = 1;

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[derive(Clone, Debug)]
struct Sha256 {
    state: [u32; 8],
}

impl Sha256 {
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
        }
    }

    // taken from:
    // https://docs.rs/sha256-rs/latest/src/sha256_rs/lib.rs.html#39
    pub fn update(&mut self, input: &[u8]) {
        let bytes = input.to_vec();
        for chunk in bytes.as_slice().chunks(64) {
            let mut w = [0; 64];

            for (w, d) in w.iter_mut().zip(chunk.iter().step_by(4)).take(16) {
                *w = u32::from_be_bytes(unsafe { *(d as *const u8 as *const [u8; 4]) });
            }

            for i in 16..64 {
                let s0: u32 =
                    w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1: u32 =
                    w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }

            let mut a = self.state[0];
            let mut b = self.state[1];
            let mut c = self.state[2];
            let mut d = self.state[3];
            let mut e = self.state[4];
            let mut f = self.state[5];
            let mut g = self.state[6];
            let mut h = self.state[7];

            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ (!e & g);
                let temp1 = h
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);

                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }

            self.state[0] = self.state[0].wrapping_add(a);
            self.state[1] = self.state[1].wrapping_add(b);
            self.state[2] = self.state[2].wrapping_add(c);
            self.state[3] = self.state[3].wrapping_add(d);
            self.state[4] = self.state[4].wrapping_add(e);
            self.state[5] = self.state[5].wrapping_add(f);
            self.state[6] = self.state[6].wrapping_add(g);
            self.state[7] = self.state[7].wrapping_add(h);
        }
    }

    pub fn state(&self) -> [u32; 8] {
        self.state
    }
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

                let query = heaps.new_execute_partial_query(query);
                current_read_offset += 1;

                reads[query_index] = query;
                let data = query.value;
                data.to_big_endian(&mut block[(query_index * 32)..(query_index * 32 + 32)]);
            }

            // run round function
            hasher.update(&block);

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
                let bytes = hasher.clone().state();
                let mut hash_as_bytes32 = [0u8; 32];
                for (chunk, state_word) in hash_as_bytes32.chunks_mut(4).zip(bytes.into_iter()) {
                    chunk.copy_from_slice(&state_word.to_be_bytes());
                }
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

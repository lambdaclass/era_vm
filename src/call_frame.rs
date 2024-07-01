use std::num::Saturating;

use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;

use crate::state::{Heap, Stack};
use crate::store::InMemory;

#[derive(Debug, Clone)]
pub struct CallFrame {
    // Max length for this is 1 << 16. Might want to enforce that at some point
    pub stack: Stack,
    pub heap: Heap,
    // Code memory is word addressable even though instructions are 64 bit wide.
    // TODO: this is a Vec of opcodes now but it's probably going to switch back to a
    // Vec<U256> later on, because I believe we have to record memory queries when
    // fetching code to execute. Check this
    pub aux_heap: Heap,
    pub code_page: Vec<U256>,
    pub pc: u64,
    /// Transient storage should be used for temporary storage within a transaction and then discarded.
    pub transient_storage: InMemory,
    pub gas_left: Saturating<u32>,
    /// The contract of this call frame's context
    pub contract_address: Address,
}
impl CallFrame {
    pub fn new(program_code: Vec<U256>, gas_stipend: u32, address: Address) -> Self {
        Self {
            stack: Stack::new(),
            heap: Heap::default(),
            aux_heap: Heap::default(),
            code_page: program_code,
            pc: 0,
            gas_left: Saturating(gas_stipend),
            transient_storage: InMemory::default(),
            contract_address: address,
        }
    }
}

use std::{cell::RefCell, num::Saturating, rc::Rc};

use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;

use crate::{
    state::{Heap, Stack},
    store::{InMemory, Storage},
};

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
    /// Storage for the frame using a type that implements the Storage trait.
    /// The supported types are InMemory and RocksDB storage.
    pub storage: Rc<RefCell<dyn Storage>>,
    /// Transient storage should be used for temporary storage within a transaction and then discarded.
    pub transient_storage: InMemory,
    pub gas_left: Saturating<u32>,
    pub context_u128: u128,
    pub this_address: Address,
    pub msg_sender: Address,
    pub code_address: Address,
    pub ergs_remaining: u32,
    pub this_shard_id: u8,
    pub caller_shard_id: u8,
    pub code_shard_id: u8,
}
impl CallFrame {
    pub fn new(
        program_code: Vec<U256>,
        gas_stipend: u32,
        storage: Rc<RefCell<dyn Storage>>,
    ) -> Self {
        Self {
            stack: Stack::new(),
            heap: Heap::default(),
            aux_heap: Heap::default(),
            code_page: program_code,
            pc: 0,
            gas_left: Saturating(gas_stipend),
            storage,
            transient_storage: InMemory::default(),
            context_u128: 0,
            this_address: Address::default(),
            msg_sender: Address::default(),
            code_address: Address::default(),
            ergs_remaining: 0,
            this_shard_id: 0,
            caller_shard_id: 0,
            code_shard_id: 0,
        }
    }
}

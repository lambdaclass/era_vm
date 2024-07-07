use std::num::Saturating;

use u256::{H160, U256};

use crate::state::{Heap, Stack};
use crate::store::InMemory;
#[derive(Debug, Clone)]
pub struct CallFrame {
    // Max length for this is 1 << 16. Might want to enforce that at some point
    pub stack: Stack,
    pub heap: Heap,
    pub aux_heap: Heap,
    // Code memory is word addressable even though instructions are 64 bit wide.
    pub code_page: Vec<U256>,
    pub pc: u64,
    /// Transient storage should be used for temporary storage within a transaction and then discarded.
    pub transient_storage: Box<InMemory>,
    pub gas_left: Saturating<u32>,
    pub exception_handler: u64,
    pub contract_address: H160,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub frame: CallFrame,
    pub near_call_frames: Vec<CallFrame>,
}

impl Context {
    pub fn new(program_code: Vec<U256>, gas_stipend: u32, contract_address: H160) -> Self {
        Self {
            frame: CallFrame::new_far_call_frame(program_code, gas_stipend, contract_address),
            near_call_frames: vec![],
        }
    }
}

impl CallFrame {
    pub fn new_far_call_frame(
        program_code: Vec<U256>,
        gas_stipend: u32,
        contract_address: H160,
    ) -> Self {
        Self {
            stack: Stack::new(),
            heap: Heap::default(),
            aux_heap: Heap::default(),
            code_page: program_code,
            pc: 0,
            // This is just a default storage, with the VMStateBuilder, you can override the storage
            gas_left: Saturating(gas_stipend),
            transient_storage: Box::new(InMemory::new_empty()),
            exception_handler: 0,
            contract_address,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_near_call_frame(
        stack: Stack,
        heap: Heap,
        aux_heap: Heap,
        code_page: Vec<U256>,
        pc: u64,
        gas_stipend: u32,
        contract_address: H160,
        transient_storage: Box<InMemory>,
        exception_handler: u64,
    ) -> Self {
        let transient_storage = transient_storage.clone();
        Self {
            stack,
            heap,
            aux_heap,
            code_page,
            pc,
            gas_left: Saturating(gas_stipend),
            transient_storage,
            contract_address,
            exception_handler,
        }
    }
}

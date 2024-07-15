use std::num::Saturating;

use u256::{H160, U256};
use zkevm_opcode_defs::ethereum_types::Address;

use crate::{state::Stack, store::InMemory};

#[derive(Debug, Clone)]
pub struct CallFrame {
    // Max length for this is 1 << 16. Might want to enforce that at some point
    pub stack: Stack,
    pub heap_id: u32,
    pub aux_heap_id: u32,
    pub calldata_heap_id: u32,
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
    /// The address of the contract being executed
    pub contract_address: Address,
    /// The address of the caller
    pub caller: Address,
    /// The address of the code being executed
    pub code_address: Address,
    /// Stands for the amount of wei sent in a transaction
    pub context_u128: u128,
}

// When someone far calls, the new frame will allocate both a new heap and a new aux heap, but not
// a new calldata. For calldata it'll pass a heap id (or a fat pointer, check this)

impl Context {
    pub fn new(
        program_code: Vec<U256>,
        gas_stipend: u32,
        contract_address: Address,
        caller: Address,
        heap_id: u32,
        aux_heap_id: u32,
        calldata_heap_id: u32,
    ) -> Self {
        Self {
            frame: CallFrame::new_far_call_frame(
                program_code,
                gas_stipend,
                contract_address,
                heap_id,
                aux_heap_id,
                calldata_heap_id,
            ),
            near_call_frames: vec![],
            contract_address,
            caller,
            code_address: contract_address,
            context_u128: 0,
        }
    }
}

impl CallFrame {
    pub fn new_far_call_frame(
        program_code: Vec<U256>,
        gas_stipend: u32,
        contract_address: H160,
        heap_id: u32,
        aux_heap_id: u32,
        calldata_heap_id: u32,
    ) -> Self {
        Self {
            stack: Stack::new(),
            heap_id,
            aux_heap_id,
            calldata_heap_id,
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
        heap_id: u32,
        aux_heap_id: u32,
        calldata_heap_id: u32,
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
            heap_id,
            aux_heap_id,
            code_page,
            calldata_heap_id,
            pc,
            gas_left: Saturating(gas_stipend),
            transient_storage,
            contract_address,
            exception_handler,
        }
    }
    pub fn resize_heap(&mut self, size: u32) -> u32 {
        self.heap.expand_memory(size)
    }
    pub fn resize_aux_heap(&mut self, size: u32) -> u32 {
        self.heap.expand_memory(size)
    }
}

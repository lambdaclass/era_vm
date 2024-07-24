use std::num::Saturating;
use u256::{H160, U256};
use zkevm_opcode_defs::ethereum_types::Address;

use crate::{state::Stack, store::InMemory};

#[derive(Debug, Clone)]
pub struct CallFrame {
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
    pub sp: u64,
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
    // Max length for this is 1 << 16. Might want to enforce that at some point
    pub stack: Stack,
}

// When someone far calls, the new frame will allocate both a new heap and a new aux heap, but not
// a new calldata. For calldata it'll pass a heap id (or a fat pointer, check this)

impl Context {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        program_code: Vec<U256>,
        gas_stipend: u32,
        contract_address: Address,
        caller: Address,
        heap_id: u32,
        aux_heap_id: u32,
        calldata_heap_id: u32,
        exception_handler: u64,
        context_u128: u128,
    ) -> Self {
        Self {
            frame: CallFrame::new_far_call_frame(
                program_code,
                gas_stipend,
                contract_address,
                heap_id,
                aux_heap_id,
                calldata_heap_id,
                exception_handler,
            ),
            near_call_frames: vec![],
            contract_address,
            caller,
            code_address: contract_address,
            context_u128,
            stack: Stack::new(),
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
        exception_handler: u64,
    ) -> Self {
        Self {
            heap_id,
            aux_heap_id,
            calldata_heap_id,
            code_page: program_code,
            pc: 0,
            gas_left: Saturating(gas_stipend),
            transient_storage: Box::new(InMemory::new_empty()),
            exception_handler,
            contract_address,
            sp: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_near_call_frame(
        sp: u64,
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
            heap_id,
            aux_heap_id,
            code_page,
            calldata_heap_id,
            pc,
            gas_left: Saturating(gas_stipend),
            transient_storage,
            contract_address,
            exception_handler,
            sp,
        }
    }
}

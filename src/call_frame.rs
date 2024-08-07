use std::num::Saturating;
use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;

use crate::{state::Stack, store::InMemory, utils::is_kernel};

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub pc: u64,
    /// Transient storage should be used for temporary storage within a transaction and then discarded.
    pub transient_storage: Box<InMemory>,
    pub gas_left: Saturating<u32>,
    pub exception_handler: u64,
    pub sp: u32,
    pub storage_before: InMemory,
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
    pub heap_id: u32,
    pub aux_heap_id: u32,
    pub calldata_heap_id: u32,
    // Code memory is word addressable even though instructions are 64 bit wide.
    pub code_page: Vec<U256>,
    pub is_static: bool,
}

// When someone far calls, the new frame will allocate both a new heap and a new aux heap, but not
// a new calldata. For calldata it'll pass a heap id (or a fat pointer, check this)

impl Context {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        program_code: Vec<U256>,
        gas_stipend: u32,
        contract_address: Address,
        code_address: Address,
        caller: Address,
        heap_id: u32,
        aux_heap_id: u32,
        calldata_heap_id: u32,
        exception_handler: u64,
        context_u128: u128,
        transient_storage: Box<InMemory>,
        storage_before: InMemory,
        is_static: bool,
    ) -> Self {
        Self {
            frame: CallFrame::new_far_call_frame(
                gas_stipend,
                exception_handler,
                storage_before,
                transient_storage,
            ),
            near_call_frames: vec![],
            contract_address,
            caller,
            code_address,
            context_u128,
            stack: Stack::new(),
            heap_id,
            aux_heap_id,
            calldata_heap_id,
            code_page: program_code,
            is_static,
        }
    }

    pub fn is_kernel(&self) -> bool {
        is_kernel(&self.contract_address)
    }
}

impl CallFrame {
    pub fn new_far_call_frame(
        gas_stipend: u32,
        exception_handler: u64,
        storage_before: InMemory,
        transient_storage: Box<InMemory>,
    ) -> Self {
        Self {
            pc: 0,
            gas_left: Saturating(gas_stipend),
            transient_storage,
            exception_handler,
            sp: 0,
            storage_before,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_near_call_frame(
        sp: u32,
        pc: u64,
        gas_stipend: u32,
        transient_storage: Box<InMemory>,
        exception_handler: u64,
        storage_before: InMemory,
    ) -> Self {
        Self {
            pc,
            gas_left: Saturating(gas_stipend),
            transient_storage,
            exception_handler,
            sp,
            storage_before,
        }
    }
}

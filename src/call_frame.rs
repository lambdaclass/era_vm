use std::num::Saturating;
use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;

use crate::{execution::Stack, state::StateSnapshot, utils::is_kernel};

#[derive(Debug, Clone, PartialEq)]
pub struct CallFrame {
    pub pc: u64,
    pub gas_left: Saturating<u32>,
    pub exception_handler: u64,
    pub sp: u32,
    pub snapshot: StateSnapshot,
    pub stipend: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodePage(Vec<U256>);

impl CodePage {
    pub fn get(&self, idx: usize) -> U256 {
        // NOTE: the spec mandates reads past the end of the program return any value that decodes
        // as an `invalid` instruction. 0u256 fits the bill because its decoded variant is 0 which
        // in turn is **the** invalid opcode.
        self.0.get(idx).cloned().unwrap_or_else(U256::zero)
    }
}

#[derive(Debug, Clone, PartialEq)]
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
    pub code_page: CodePage,
    pub is_static: bool,
}

// When someone far calls, the new frame will allocate both a new heap and a new aux heap, but not
// a new calldata. For calldata it'll pass a heap id (or a fat pointer, check this)

impl Context {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        program_code: Vec<U256>,
        gas: u32,
        contract_address: Address,
        code_address: Address,
        caller: Address,
        heap_id: u32,
        aux_heap_id: u32,
        calldata_heap_id: u32,
        exception_handler: u64,
        context_u128: u128,
        snapshot: StateSnapshot,
        is_static: bool,
        stipend: u32,
    ) -> Self {
        Self {
            frame: CallFrame::new_far_call_frame(gas, stipend, exception_handler, snapshot),
            near_call_frames: vec![],
            contract_address,
            caller,
            code_address,
            context_u128,
            stack: Stack::new(),
            heap_id,
            aux_heap_id,
            calldata_heap_id,
            code_page: CodePage(program_code),
            is_static,
        }
    }

    pub fn is_kernel(&self) -> bool {
        is_kernel(&self.contract_address)
    }
}

impl CallFrame {
    pub fn new_far_call_frame(
        gas: u32,
        stipend: u32,
        exception_handler: u64,
        snapshot: StateSnapshot,
    ) -> Self {
        Self {
            pc: 0,
            stipend: stipend,
            gas_left: Saturating(gas),
            exception_handler,
            sp: 0,
            snapshot,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_near_call_frame(
        sp: u32,
        pc: u64,
        gas: u32,
        exception_handler: u64,
        snapshot: StateSnapshot,
    ) -> Self {
        Self {
            pc,
            gas_left: Saturating(gas),
            exception_handler,
            stipend: 0,
            sp,
            snapshot,
        }
    }
}

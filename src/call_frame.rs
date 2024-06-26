use std::{cell::RefCell, num::Saturating, rc::Rc};

use u256::U256;

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
    // TODO: Storage is more complicated than this. We probably want to abstract it into a trait
    // to support in-memory vs on-disk storage, etc.
    pub storage: Rc<RefCell<dyn Storage>>,
    pub gas_left: Saturating<u32>,
    pub transient_storage: InMemory,
    pub exception_handler: u64,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub frame: CallFrame,
    pub near_call_frames: Vec<CallFrame>,
}

impl Context {
    pub fn new(program_code: Vec<U256>, gas_stipend: u32) -> Self {
        Self {
            frame: CallFrame::new_far_call_frame(program_code, gas_stipend),
            near_call_frames: vec![],
        }
    }
}

impl CallFrame {
    pub fn new_far_call_frame(program_code: Vec<U256>, gas_stipend: u32) -> Self {
        Self {
            stack: Stack::new(),
            heap: Heap::default(),
            aux_heap: Heap::default(),
            code_page: program_code,
            pc: 0,
            storage: Rc::new(RefCell::new(InMemory::default())),
            gas_left: Saturating(gas_stipend),
            transient_storage: InMemory::default(),
            exception_handler: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_near_call_frame(
        stack: Stack,
        heap: Heap,
        aux_heap: Heap,
        code_page: Vec<U256>,
        pc: u64,
        storage: Rc<RefCell<dyn Storage>>,
        gas_stipend: u32,
        transient_storage: InMemory,
        exception_handler: u64,
    ) -> Self {
        Self {
            stack,
            heap,
            aux_heap,
            code_page,
            pc,
            storage,
            gas_left: Saturating(gas_stipend),
            transient_storage,
            exception_handler,
        }
    }
}

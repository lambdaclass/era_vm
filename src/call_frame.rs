use std::{collections::HashMap, num::Saturating};

use u256::U256;

use crate::state::Stack;

#[derive(Debug, Clone)]
pub struct CallFrame {
    // Max length for this is 1 << 16. Might want to enforce that at some point
    pub stack: Stack,
    pub heap: Vec<U256>,
    // Code memory is word addressable even though instructions are 64 bit wide.
    // TODO: this is a Vec of opcodes now but it's probably going to switch back to a
    // Vec<U256> later on, because I believe we have to record memory queries when
    // fetching code to execute. Check this
    pub code_page: Vec<U256>,
    pub pc: u64,
    // TODO: Storage is more complicated than this. We probably want to abstract it into a trait
    // to support in-memory vs on-disk storage, etc.
    pub storage: HashMap<U256, U256>,
    pub gas_left: Saturating<u32>,
}

impl CallFrame {
    pub fn new(program_code: Vec<U256>, gas_stipend: u32) -> Self {
        Self {
            stack: Stack::new(),
            heap: vec![],
            code_page: program_code,
            pc: 0,
            storage: HashMap::new(),
            gas_left: Saturating(gas_stipend),
        }
    }
}

use std::collections::HashMap;

use crate::{opcode::Opcode, value::TaggedValue};
use u256::U256;

#[derive(Debug)]
pub struct CallFrame {
    // Max length for this is 1 << 16. Might want to enforce that at some point
    pub stack: Vec<TaggedValue>,
    pub heap: Vec<U256>,
    // Code memory is word addressable even though instructions are 64 bit wide.
    // TODO: this is a Vec of opcodes now but it's probably going to switch back to a
    // Vec<U256> later on, because I believe we have to record memory queries when
    // fetching code to execute. Check this
    pub code_page: Vec<Opcode>,
    pub pc: u64,
    // TODO: Storage is more complicated than this. We probably want to abstract it into a trait
    // to support in-memory vs on-disk storage, etc.
    pub storage: HashMap<U256, U256>,
}

#[derive(Debug)]
pub struct VMState {
    // The first register, r0, is actually always zero and not really used.
    // Writing to it does nothing.
    pub registers: [U256; 15],
    pub flags: u8, // We only use the first three bits for the flags here: LT, GT, EQ.
    pub current_frame: CallFrame,
}

impl VMState {
    // TODO: The VM will probably not take the program to execute as a parameter later on.
    pub fn new(program_code: Vec<Opcode>) -> Self {
        Self {
            registers: [U256::zero(); 15],
            flags: 0,
            current_frame: CallFrame::new(program_code),
        }
    }

    pub fn get_register(&self, index: u8) -> U256 {
        if index != 0 {
            return self.registers[(index - 1) as usize];
        }

        return U256::zero();
    }

    pub fn set_register(&mut self, index: u8, value: U256) {
        if index == 0 {
            return;
        }

        self.registers[(index - 1) as usize] = value;
    }
}

impl CallFrame {
    pub fn new(program_code: Vec<Opcode>) -> Self {
        Self {
            stack: vec![],
            heap: vec![],
            code_page: program_code,
            pc: 0,
            storage: HashMap::new(),
        }
    }
}

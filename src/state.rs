use std::collections::HashMap;

use crate::{value::TaggedValue, Opcode};
use u256::U256;
use zkevm_opcode_defs::OpcodeVariant;

#[derive(Debug)]
pub struct Stack {
    pub stack: Vec<TaggedValue>,
}

#[derive(Debug)]
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
    pub fn new(program_code: Vec<U256>) -> Self {
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

        U256::zero()
    }

    pub fn set_register(&mut self, index: u8, value: U256) {
        if index == 0 {
            return;
        }

        self.registers[(index - 1) as usize] = value;
    }

    pub fn get_opcode(&self, opcode_table: &[OpcodeVariant]) -> Opcode {
        let raw_opcode = self.current_frame.code_page[(self.current_frame.pc / 4) as usize];
        let raw_opcode_64 = match self.current_frame.pc % 4 {
            3 => (raw_opcode & u64::MAX.into()).as_u64(),
            2 => ((raw_opcode >> 64) & u64::MAX.into()).as_u64(),
            1 => ((raw_opcode >> 128) & u64::MAX.into()).as_u64(),
            0 => ((raw_opcode >> 192) & u64::MAX.into()).as_u64(),
            _ => panic!("This should never happen"),
        };

        Opcode::from_raw_opcode(raw_opcode_64, opcode_table)
    }
}

impl CallFrame {
    pub fn new(program_code: Vec<U256>) -> Self {
        Self {
            stack: Stack::new(),
            heap: vec![],
            code_page: program_code,
            pc: 0,
            storage: HashMap::new(),
        }
    }
}

impl Stack {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn push(&mut self, value: TaggedValue) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) -> TaggedValue {
        self.stack.pop().unwrap()
    }

    pub fn sp(&self) -> usize {
        self.stack.len()
    }

    pub fn get_with_offset(&self, offset: usize) -> &TaggedValue {
        &self.stack[self.sp() - offset]
    }

    pub fn get_absolute(&self, index: usize) -> &TaggedValue {
        &self.stack[index]
    }

    pub fn store_with_offset(&mut self, offset: usize, value: TaggedValue) {
        let sp = self.sp();
        self.stack[sp - offset] = value;
    }

    pub fn store_absolute(&mut self, index: usize, value: TaggedValue) {
        if index >= self.sp() { // What to do if its not inmediately after sp? Fill with 0s?
            self.stack.push(value);
        } else {
            self.stack[index] = value;
        }
    }
}

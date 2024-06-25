use std::{collections::HashMap, num::Saturating};

use crate::{opcode::Predicate, value::TaggedValue, Opcode};
use u256::U256;
use zkevm_opcode_defs::OpcodeVariant;

#[derive(Debug, Clone)]
pub struct Stack {
    pub stack: Vec<TaggedValue>,
}

// I'm not really a fan of this, but it saves up time when
// adding new fields to the vm state, and makes it easier
// to setup certain particular state for the tests .
#[derive(Debug, Clone)]
pub struct VMStateBuilder {
    pub registers: [TaggedValue; 15],
    pub flag_lt_of: bool,
    pub flag_gt: bool,
    pub flag_eq: bool,
    pub running_frames: Vec<CallFrame>,
}

impl Default for VMStateBuilder {
    fn default() -> Self {
        VMStateBuilder {
            registers: [TaggedValue::default(); 15],
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            running_frames: vec![],
        }
    }
}
impl VMStateBuilder {
    pub fn new() -> VMStateBuilder {
        Default::default()
    }
    pub fn with_registers(mut self, registers: [TaggedValue; 15]) -> VMStateBuilder {
        self.registers = registers;
        self
    }
    pub fn with_frames(mut self, frame: Vec<CallFrame>) -> VMStateBuilder {
        self.running_frames = frame;
        self
    }

    pub fn eq_flag(mut self, eq: bool) -> VMStateBuilder {
        self.flag_eq = eq;
        self
    }
    pub fn gt_flag(mut self, gt: bool) -> VMStateBuilder {
        self.flag_gt = gt;
        self
    }
    pub fn lt_of_flag(mut self, lt_of: bool) -> VMStateBuilder {
        self.flag_lt_of = lt_of;
        self
    }

    pub fn build(self) -> VMState {
        VMState {
            registers: self.registers,
            running_frames: self.running_frames,
            flag_eq: self.flag_eq,
            flag_gt: self.flag_gt,
            flag_lt_of: self.flag_lt_of,
        }
    }
}
#[derive(Debug, Clone)]
pub struct VMState {
    // The first register, r0, is actually always zero and not really used.
    // Writing to it does nothing.
    pub registers: [TaggedValue; 15],
    /// Overflow or less than flag
    pub flag_lt_of: bool, // We only use the first three bits for the flags here: LT, GT, EQ.
    /// Greater Than flag
    pub flag_gt: bool,
    /// Equal flag
    pub flag_eq: bool,
    pub running_frames: Vec<CallFrame>,
}

impl Default for VMState {
    fn default() -> Self {
        Self::new()
    }
}

// Totally arbitrary, probably we will have to change it later.
pub const DEFAULT_INITIAL_GAS: u32 = 1 << 16;
impl VMState {
    // TODO: The VM will probably not take the program to execute as a parameter later on.
    pub fn new() -> Self {
        Self {
            registers: [TaggedValue::default(); 15],
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            running_frames: vec![],
        }
    }

    pub fn load_program(&mut self, program_code: Vec<U256>) {
        self.push_frame(program_code, DEFAULT_INITIAL_GAS);
    }

    pub fn push_frame(&mut self, program_code: Vec<U256>, gas_stipend: u32) {
        if let Some(frame) = self.running_frames.last_mut() {
            frame.gas_left -= Saturating(gas_stipend)
        }
        let new_context = CallFrame::new(program_code, gas_stipend);
        self.running_frames.push(new_context);
    }
    pub fn pop_frame(&mut self) {
        self.running_frames.pop();
    }
    pub fn current_context_mut(&mut self) -> &mut CallFrame {
        self.running_frames
            .last_mut()
            .expect("Fatal: VM has no running contract")
    }

    pub fn current_context(&self) -> &CallFrame {
        self.running_frames
            .last()
            .expect("Fatal: VM has no running contract")
    }

    pub fn predicate_holds(&self, condition: &Predicate) -> bool {
        match condition {
            Predicate::Always => true,
            Predicate::Gt => self.flag_gt,
            Predicate::Lt => self.flag_lt_of,
            Predicate::Eq => self.flag_eq,
            Predicate::Ge => self.flag_eq || self.flag_gt,
            Predicate::Le => self.flag_eq || self.flag_lt_of,
            Predicate::Ne => !self.flag_eq,
            Predicate::GtOrLt => self.flag_gt || self.flag_lt_of,
        }
    }

    pub fn get_register(&self, index: u8) -> TaggedValue {
        if index != 0 {
            return self.registers[(index - 1) as usize];
        }

        TaggedValue::default()
    }

    pub fn set_register(&mut self, index: u8, value: TaggedValue) {
        if index == 0 {
            return;
        }

        self.registers[(index - 1) as usize] = value;
    }

    pub fn get_opcode(&self, opcode_table: &[OpcodeVariant]) -> Opcode {
        let current_context = self.current_context();
        let pc = current_context.pc;
        let raw_opcode = current_context.code_page[(pc / 4) as usize];
        let raw_opcode_64 = match pc % 4 {
            3 => (raw_opcode & u64::MAX.into()).as_u64(),
            2 => ((raw_opcode >> 64) & u64::MAX.into()).as_u64(),
            1 => ((raw_opcode >> 128) & u64::MAX.into()).as_u64(),
            0 => ((raw_opcode >> 192) & u64::MAX.into()).as_u64(),
            _ => panic!("This should never happen"),
        };

        Opcode::from_raw_opcode(raw_opcode_64, opcode_table)
    }

    pub fn decrease_gas(&mut self, opcode: &Opcode) {
        self.current_context_mut().gas_left -= opcode.variant.ergs_price();
    }
}

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

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn push(&mut self, value: TaggedValue) {
        self.stack.push(value);
    }

    pub fn fill_with_zeros(&mut self, value: U256) {
        for _ in 0..value.as_usize() {
            self.stack.push(TaggedValue {
                value: U256::zero(),
                is_pointer: false,
            });
        }
    }

    pub fn pop(&mut self, value: U256) {
        for _ in 0..value.as_usize() {
            self.stack.pop().unwrap();
        }
    }

    pub fn sp(&self) -> usize {
        self.stack.len()
    }

    pub fn get_with_offset(&self, offset: usize) -> &TaggedValue {
        let sp = self.sp();
        if offset > sp || offset == 0 {
            panic!("Trying to read outside of stack bounds");
        }
        &self.stack[sp - offset]
    }

    pub fn get_absolute(&self, index: usize) -> &TaggedValue {
        if index >= self.sp() {
            panic!("Trying to read outside of stack bounds");
        }
        &self.stack[index]
    }

    pub fn store_with_offset(&mut self, offset: usize, value: TaggedValue) {
        let sp = self.sp();
        if offset > sp || offset == 0 {
            panic!("Trying to store outside of stack bounds");
        }
        self.stack[sp - offset] = value;
    }

    pub fn store_absolute(&mut self, index: usize, value: TaggedValue) {
        if index >= self.sp() {
            panic!("Trying to store outside of stack bounds");
        }
        self.stack[index] = value;
    }
}

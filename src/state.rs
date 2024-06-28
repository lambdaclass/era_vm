use std::collections::HashMap;
use std::default;
use std::num::Saturating;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::call_frame::CallFrame;
use crate::{
    opcode::Predicate,
    store::{InMemory, RocksDB, Storage},
    value::{FatPointer, TaggedValue},
    Opcode,
};
use u256::{H160, U256};
use zkevm_opcode_defs::{OpcodeVariant, MEMORY_GROWTH_ERGS_PER_BYTE};

#[derive(Debug, Clone)]
pub struct Stack {
    pub stack: Vec<TaggedValue>,
}

#[derive(Debug, Clone)]
pub struct Heap {
    heap: Vec<u8>,
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
    pub storage: Rc<dyn Storage>,
}

// On this specific struct, I prefer to have the actual values
// instead of guessing which ones are the defaults.
#[allow(clippy::derivable_impls)]
impl Default for VMStateBuilder {
    fn default() -> Self {
        VMStateBuilder {
            registers: [TaggedValue::default(); 15],
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            running_frames: vec![],
            storage: Rc::new(InMemory::new_empty()),
        }
    }
}
impl VMStateBuilder {
    pub fn new() -> VMStateBuilder {
        Default::default()
    }
    pub fn with_storage(mut self, storage: Rc<dyn Storage>) -> VMStateBuilder {
        self.storage = storage.clone();
        self
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
            storage: self.storage,
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
    pub storage: Rc<dyn Storage>,
}

impl Default for VMState {
    fn default() -> Self {
        Self::new()
    }
}

// Arbitrary default, change it if you need to.
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
            storage: Rc::new(InMemory::new_empty()),
        }
    }

    pub fn load_program(
        &mut self,
        program_code: Vec<U256>,
        contract_address: H160,
    ) {
        self.push_frame(program_code, DEFAULT_INITIAL_GAS, contract_address);
    }

    pub fn push_frame(
        &mut self,
        program_code: Vec<U256>,
        gas_stipend: u32,
        address: H160,
    ) {
        if let Some(frame) = self.running_frames.last_mut() {
            frame.gas_left -= Saturating(gas_stipend)
        }
        // TODO: Properly implement this.
        let new_context = CallFrame::new(program_code, gas_stipend, self.storage.clone(), address);
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

    pub(crate) fn decommit_from_address(&self, contract_address: &H160) -> Vec<U256> {
        let hash = self
            .storage
            .get_contract_hash(contract_address)
            .expect("Fatal: contract does not exist");
        self
            .storage
            .get_contract_code(&hash)
            .expect("Fatal: hash found but it does not have an associated contract")
    }

    pub fn decommit(&mut self, contract_hash: &U256) -> Vec<U256> {
        // TODO: Do the proper decommit operation
        self.storage
            .get_contract_code(contract_hash)
            .expect("Fatal: contract does not exist")
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

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    pub fn new() -> Self {
        Self { heap: vec![] }
    }
    // Returns how many ergs the expand costs
    pub fn expand_memory(&mut self, address: u32) -> u32 {
        if address >= self.heap.len() as u32 {
            let old_size = self.heap.len() as u32;
            self.heap.resize(address as usize + 1, 0);
            return MEMORY_GROWTH_ERGS_PER_BYTE * (address - old_size + 1);
        }
        0
    }

    pub fn store(&mut self, address: u32, value: U256) {
        let mut bytes: [u8; 32] = [0; 32];
        value.to_big_endian(&mut bytes);
        for (i, item) in bytes.iter().enumerate() {
            self.heap[address as usize + i] = *item;
        }
    }

    pub fn read(&mut self, address: u32) -> U256 {
        let mut result = U256::zero();
        for i in 0..32 {
            result |= U256::from(self.heap[address as usize + (31 - i)]) << (i * 8);
        }
        result
    }

    pub fn read_from_pointer(&mut self, pointer: &FatPointer) -> U256 {
        let mut result = U256::zero();
        for i in 0..32 {
            let addr = pointer.start + pointer.offset + (31 - i);
            if addr < pointer.start + pointer.len {
                result |= U256::from(self.heap[addr as usize]) << (i * 8);
            }
        }
        result
    }
}

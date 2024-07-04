use std::num::Saturating;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::eravm_error::EraVmError;
use crate::store::RocksDB;
use crate::{
    call_frame::{CallFrame, Context},
    opcode::Predicate,
    value::{FatPointer, TaggedValue},
    Opcode,
};
use u256::U256;
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
#[derive(Debug, Clone, Default)]
pub struct VMStateBuilder {
    pub registers: [TaggedValue; 15],
    pub flag_lt_of: bool,
    pub flag_gt: bool,
    pub flag_eq: bool,
    pub running_contexts: Vec<Context>,
}

impl VMStateBuilder {
    pub fn new() -> VMStateBuilder {
        Default::default()
    }
    pub fn with_registers(mut self, registers: [TaggedValue; 15]) -> VMStateBuilder {
        self.registers = registers;
        self
    }
    pub fn with_contexts(mut self, contexts: Vec<Context>) -> VMStateBuilder {
        self.running_contexts = contexts;
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
    pub fn with_storage(mut self, storage: PathBuf) -> Result<VMStateBuilder, EraVmError> {
        let storage = Rc::new(RefCell::new(RocksDB::open(storage)?));
        if self.running_contexts.is_empty() {
            self.running_contexts
                .push(Context::new(vec![], DEFAULT_INITIAL_GAS));
        }
        for context in self.running_contexts.iter_mut() {
            context.frame.storage = storage.clone();
            for frame in context.near_call_frames.iter_mut() {
                frame.storage = storage.clone();
            }
        }
        Ok(self)
    }
    pub fn build(self) -> VMState {
        VMState {
            registers: self.registers,
            running_contexts: self.running_contexts,
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
    pub running_contexts: Vec<Context>,
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
            running_contexts: vec![],
        }
    }

    pub fn load_program(&mut self, program_code: Vec<U256>) {
        if self.running_contexts.is_empty() {
            self.push_far_call_frame(program_code, DEFAULT_INITIAL_GAS);
        } else {
            for context in self.running_contexts.iter_mut() {
                if context.frame.code_page.is_empty() {
                    context.frame.code_page.clone_from(&program_code);
                }
                for frame in context.near_call_frames.iter_mut() {
                    if frame.code_page.is_empty() {
                        frame.code_page.clone_from(&program_code);
                    }
                }
            }
        }
    }

    pub fn push_far_call_frame(&mut self, program_code: Vec<U256>, gas_stipend: u32) {
        if let Some(context) = self.running_contexts.last_mut() {
            context.frame.gas_left -= Saturating(gas_stipend)
        }
        let new_context = Context::new(program_code, gas_stipend);
        self.running_contexts.push(new_context);
    }
    pub fn pop_context(&mut self) -> Result<Context, EraVmError> {
        self.running_contexts.pop().ok_or(EraVmError::ContextError(
            "VM has no running contract".to_string(),
        ))
    }

    pub fn pop_frame(&mut self) -> Result<CallFrame, EraVmError> {
        let current_context = self.current_context_mut()?;
        if current_context.near_call_frames.is_empty() {
            let context = self.pop_context()?;
            Ok(context.frame)
        } else {
            current_context
                .near_call_frames
                .pop()
                .ok_or(EraVmError::ContextError(
                    "VM has no running contract".to_string(),
                ))
        }
    }

    pub fn push_near_call_frame(&mut self, near_call_frame: CallFrame) -> Result<(), EraVmError> {
        self.current_context_mut()?
            .near_call_frames
            .push(near_call_frame);
        Ok(())
    }

    pub fn current_context_mut(&mut self) -> Result<&mut Context, EraVmError> {
        self.running_contexts
            .last_mut()
            .ok_or(EraVmError::ContextError(
                "VM has no running contract".to_string(),
            ))
    }

    pub fn current_context(&self) -> Result<&Context, EraVmError> {
        self.running_contexts.last().ok_or(EraVmError::ContextError(
            "VM has no running contract".to_string(),
        ))
    }

    pub fn current_frame_mut(&mut self) -> Result<&mut CallFrame, EraVmError> {
        let current_context = self.current_context_mut()?;
        if current_context.near_call_frames.is_empty() {
            Ok(&mut current_context.frame)
        } else {
            current_context
                .near_call_frames
                .last_mut()
                .ok_or(EraVmError::ContextError(
                    "VM has no running contract".to_string(),
                ))
        }
    }

    pub fn current_frame(&self) -> Result<&CallFrame, EraVmError> {
        let current_context = self.current_context()?;
        if current_context.near_call_frames.is_empty() {
            Ok(&current_context.frame)
        } else {
            current_context
                .near_call_frames
                .last()
                .ok_or(EraVmError::ContextError(
                    "VM has no running contract".to_string(),
                ))
        }
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

    pub fn get_opcode(&self, opcode_table: &[OpcodeVariant]) -> Result<Opcode, EraVmError> {
        let current_context = self.current_frame()?;
        let pc = current_context.pc;
        let raw_opcode = current_context.code_page[(pc / 4) as usize];
        let raw_opcode_64 = match pc % 4 {
            3 => (raw_opcode & u64::MAX.into()).as_u64(),
            2 => ((raw_opcode >> 64) & u64::MAX.into()).as_u64(),
            1 => ((raw_opcode >> 128) & u64::MAX.into()).as_u64(),
            0 => ((raw_opcode >> 192) & u64::MAX.into()).as_u64(),
            _ => panic!("This should never happen"),
        };

        Ok(Opcode::from_raw_opcode(raw_opcode_64, opcode_table))
    }

    pub fn decrease_gas(&mut self, opcode: &Opcode) -> Result<bool, EraVmError> {
        let underflows = opcode.variant.ergs_price() > self.current_frame()?.gas_left.0; // Return true if underflows
        self.current_frame_mut()?.gas_left -= opcode.variant.ergs_price();
        Ok(underflows)
    }

    pub fn set_gas_left(&mut self, gas: u32) -> Result<(), EraVmError> {
        self.current_frame_mut()?.gas_left = Saturating(gas);
        Ok(())
    }

    pub fn gas_left(&self) -> Result<u32, EraVmError> {
        Ok(self.current_frame()?.gas_left.0)
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

    pub fn pop(&mut self, value: U256) -> Result<(), EraVmError> {
        for _ in 0..value.as_usize() {
            self.stack
                .pop()
                .ok_or(EraVmError::ContextError("Stack underflow".to_string()))?;
        }
        Ok(())
    }

    pub fn sp(&self) -> usize {
        self.stack.len()
    }

    pub fn get_with_offset(&self, offset: usize) -> Result<&TaggedValue, EraVmError> {
        let sp = self.sp();
        if offset > sp || offset == 0 {
            return Err(EraVmError::StackError(
                "Trying to read outside of stack bounds".to_string(),
            ));
        }
        Ok(&self.stack[sp - offset])
    }

    pub fn get_absolute(&self, index: usize) -> Result<&TaggedValue, EraVmError> {
        if index >= self.sp() {
            return Err(EraVmError::StackError(
                "Trying to read outside of stack bounds".to_string(),
            ));
        }
        Ok(&self.stack[index])
    }

    pub fn store_with_offset(
        &mut self,
        offset: usize,
        value: TaggedValue,
    ) -> Result<(), EraVmError> {
        let sp = self.sp();
        if offset > sp || offset == 0 {
            return Err(EraVmError::StackError(
                "Trying to store outside of stack bounds".to_string(),
            ));
        }
        self.stack[sp - offset] = value;
        Ok(())
    }

    pub fn store_absolute(&mut self, index: usize, value: TaggedValue) -> Result<(), EraVmError> {
        if index >= self.sp() {
            return Err(EraVmError::StackError(
                "Trying to store outside of stack bounds".to_string(),
            ));
        }
        self.stack[index] = value;
        Ok(())
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

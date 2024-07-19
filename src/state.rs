use std::num::Saturating;

use crate::call_frame::{CallFrame, Context};
use crate::heaps::Heaps;

use crate::eravm_error::{ContextError, EraVmError, StackError};
use crate::{
    opcode::Predicate,
    value::{FatPointer, TaggedValue},
    Opcode,
};
use u256::{H160, U256};
use zkevm_opcode_defs::ethereum_types::Address;
use zkevm_opcode_defs::{OpcodeVariant, MEMORY_GROWTH_ERGS_PER_BYTE};

pub const CALLDATA_HEAP: u32 = 1;
pub const FIRST_HEAP: u32 = 2;
pub const FIRST_AUX_HEAP: u32 = 3;

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
#[derive(Debug)]
pub struct VMStateBuilder {
    pub registers: [TaggedValue; 15],
    pub flag_lt_of: bool,
    pub flag_gt: bool,
    pub flag_eq: bool,
    pub running_contexts: Vec<Context>,
    pub program: Vec<U256>,
    pub tx_number: u64,
    pub heaps: Heaps,
    pub events: Vec<Event>,
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
            running_contexts: vec![],
            program: vec![],
            tx_number: 0,
            heaps: Heaps::default(),
            events: vec![],
        }
    }
}
impl VMStateBuilder {
    pub fn new() -> VMStateBuilder {
        Default::default()
    }

    pub fn with_program(mut self, program: Vec<U256>) -> VMStateBuilder {
        self.program = program;
        self
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

    pub fn with_tx_number(mut self, tx_number: u64) -> VMStateBuilder {
        self.tx_number = tx_number;
        self
    }

    pub fn with_heaps(mut self, heaps: Heaps) -> VMStateBuilder {
        self.heaps = heaps;
        self
    }

    pub fn with_events(mut self, events: Vec<Event>) -> VMStateBuilder {
        self.events = events;
        self
    }

    pub fn build(self) -> VMState {
        VMState {
            registers: self.registers,
            running_contexts: self.running_contexts,
            flag_eq: self.flag_eq,
            flag_gt: self.flag_gt,
            flag_lt_of: self.flag_lt_of,
            program: self.program,
            tx_number: self.tx_number,
            heaps: self.heaps,
            events: self.events,
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
    pub program: Vec<U256>,
    pub tx_number: u64,
    pub heaps: Heaps,
    pub events: Vec<Event>,
}

// Totally arbitrary, probably we will have to change it later.
pub const DEFAULT_INITIAL_GAS: u32 = 1 << 16;
impl VMState {
    // TODO: The VM will probably not take the program to execute as a parameter later on.
    pub fn new(
        program_code: Vec<U256>,
        calldata: Vec<u8>,
        contract_address: H160,
        caller: H160,
    ) -> Self {
        let mut registers = [TaggedValue::default(); 15];
        let calldata_ptr = FatPointer {
            page: CALLDATA_HEAP,
            offset: 0,
            start: 0,
            len: calldata.len() as u32,
        };

        registers[0] = TaggedValue::new_pointer(calldata_ptr.encode());

        let context = Context::new(
            program_code.clone(),
            u32::MAX - 0x80000000,
            contract_address,
            caller,
            FIRST_HEAP,
            FIRST_AUX_HEAP,
            CALLDATA_HEAP,
            0,
            0,
        );

        let heaps = Heaps::new(calldata);

        Self {
            registers,
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            running_contexts: vec![context],
            program: program_code,
            tx_number: 0,
            heaps,
            events: vec![],
        }
    }

    /// This function is currently for tests only and should be removed.
    pub fn load_program(&mut self, program_code: Vec<U256>) {
        if self.running_contexts.is_empty() {
            self.push_far_call_frame(
                program_code,
                DEFAULT_INITIAL_GAS,
                Address::default(),
                Address::default(),
                FIRST_HEAP,
                FIRST_AUX_HEAP,
                CALLDATA_HEAP,
                0,
                0,
            );
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

    pub fn clear_registers(&mut self) {
        for register in self.registers.iter_mut() {
            *register = TaggedValue::new_raw_integer(U256::zero());
        }
    }

    pub fn clear_flags(&mut self) {
        self.flag_lt_of = false;
        self.flag_gt = false;
        self.flag_eq = false;
    }

    pub fn clear_pointer_flags(&mut self) {
        for register in self.registers.iter_mut() {
            register.to_raw_integer();
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: check if we can avoid this
    pub fn push_far_call_frame(
        &mut self,
        program_code: Vec<U256>,
        gas_stipend: u32,
        address: Address,
        caller: Address,
        heap_id: u32,
        aux_heap_id: u32,
        calldata_heap_id: u32,
        exception_handler: u64,
        context_u128: u128,
    ) {
        if let Some(context) = self.running_contexts.last_mut() {
            context.frame.gas_left -= Saturating(gas_stipend)
        }
        let new_context = Context::new(
            program_code,
            gas_stipend,
            address,
            caller,
            heap_id,
            aux_heap_id,
            calldata_heap_id,
            exception_handler,
            context_u128,
        );
        self.running_contexts.push(new_context);
    }
    pub fn pop_context(&mut self) -> Result<Context, ContextError> {
        self.running_contexts.pop().ok_or(ContextError::NoContract)
    }

    pub fn pop_frame(&mut self) -> Result<CallFrame, ContextError> {
        let current_context = self.current_context_mut()?;
        if current_context.near_call_frames.is_empty() {
            let context = self.pop_context()?;
            Ok(context.frame)
        } else {
            current_context
                .near_call_frames
                .pop()
                .ok_or(ContextError::NoContract)
        }
    }

    pub fn push_near_call_frame(&mut self, near_call_frame: CallFrame) -> Result<(), EraVmError> {
        self.current_context_mut()?
            .near_call_frames
            .push(near_call_frame);
        Ok(())
    }

    pub fn current_context_mut(&mut self) -> Result<&mut Context, ContextError> {
        self.running_contexts
            .last_mut()
            .ok_or(ContextError::NoContract)
    }

    pub fn current_context(&self) -> Result<&Context, ContextError> {
        self.running_contexts.last().ok_or(ContextError::NoContract)
    }

    pub fn current_frame_mut(&mut self) -> Result<&mut CallFrame, ContextError> {
        let current_context = self.current_context_mut()?;
        if current_context.near_call_frames.is_empty() {
            Ok(&mut current_context.frame)
        } else {
            current_context
                .near_call_frames
                .last_mut()
                .ok_or(ContextError::NoContract)
        }
    }

    pub fn current_frame(&self) -> Result<&CallFrame, ContextError> {
        let current_context = self.current_context()?;
        if current_context.near_call_frames.is_empty() {
            Ok(&current_context.frame)
        } else {
            current_context
                .near_call_frames
                .last()
                .ok_or(ContextError::NoContract)
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
            _ => ((raw_opcode >> 192) & u64::MAX.into()).as_u64(), // 0
        };

        Ok(Opcode::from_raw_opcode(raw_opcode_64, opcode_table))
    }

    pub fn decrease_gas(&mut self, cost: u32) -> Result<bool, EraVmError> {
        let underflows = cost > self.current_frame()?.gas_left.0; // Return true if underflows
        self.current_frame_mut()?.gas_left -= cost;
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

    pub fn pop(&mut self, value: U256) -> Result<(), StackError> {
        for _ in 0..value.as_usize() {
            self.stack.pop().ok_or(StackError::Underflow)?;
        }
        Ok(())
    }

    pub fn sp(&self) -> usize {
        self.stack.len()
    }

    pub fn get_with_offset(&self, offset: usize) -> Result<&TaggedValue, StackError> {
        let sp = self.sp();
        if offset > sp || offset == 0 {
            return Err(StackError::ReadOutOfBounds);
        }
        Ok(&self.stack[sp - offset])
    }

    pub fn get_absolute(&self, index: usize) -> Result<&TaggedValue, StackError> {
        if index >= self.sp() {
            return Err(StackError::ReadOutOfBounds);
        }
        Ok(&self.stack[index])
    }

    pub fn store_with_offset(
        &mut self,
        offset: usize,
        value: TaggedValue,
    ) -> Result<(), StackError> {
        let sp = self.sp();
        if offset > sp || offset == 0 {
            return Err(StackError::StoreOutOfBounds);
        }
        self.stack[sp - offset] = value;
        Ok(())
    }

    pub fn store_absolute(&mut self, index: usize, value: TaggedValue) -> Result<(), StackError> {
        if index >= self.sp() {
            return Err(StackError::StoreOutOfBounds);
        }
        self.stack[index] = value;
        Ok(())
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl Heap {
    pub fn new(values: Vec<u8>) -> Self {
        Self { heap: values }
    }
    // Returns how many ergs the expand costs
    pub fn expand_memory(&mut self, address: u32) -> u32 {
        if address >= self.heap.len() as u32 {
            let old_size = self.heap.len() as u32;
            self.heap.resize(address as usize, 0);
            return MEMORY_GROWTH_ERGS_PER_BYTE * (address - old_size);
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

    pub fn read(&self, address: u32) -> U256 {
        let mut result = U256::zero();
        for i in 0..32 {
            result |= U256::from(self.heap[address as usize + (31 - i)]) << (i * 8);
        }
        result
    }

    pub fn read_byte(&self, address: u32) -> u8 {
        self.heap[address as usize]
    }

    pub fn read_from_pointer(&self, pointer: &FatPointer) -> U256 {
        let mut result = U256::zero();
        for i in 0..32 {
            let addr = pointer.start + pointer.offset + (31 - i);
            if addr < pointer.start + pointer.len {
                result |= U256::from(self.heap[addr as usize]) << (i * 8);
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub key: U256,
    pub value: U256,
    pub is_first: bool,
    pub shard_id: u8,
    pub tx_number: u16,
}

use std::{cell::RefCell, path::PathBuf, rc::Rc};
use std::{collections::HashMap, num::Saturating};

use crate::{
    opcode::Predicate,
    store::{InMemory, RocksDB, Storage},
    value::TaggedValue,
    Opcode,
};
use u256::U256;
use zkevm_opcode_defs::OpcodeVariant;

#[derive(Debug, Clone)]
pub struct Stack {
    pub stack: Vec<TaggedValue>,
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
    /// Storage for the frame using a type that implements the Storage trait.
    /// The supported types are InMemory and RocksDB storage.
    pub storage: Rc<RefCell<dyn Storage>>,
    pub transient_storage: InMemory,
}

#[derive(Debug, Clone)]
pub struct VMState {
    // The first register, r0, is actually always zero and not really used.
    // Writing to it does nothing.
    pub registers: [U256; 15],
    /// Overflow or less than flag
    pub flag_lt_of: bool, // We only use the first three bits for the flags here: LT, GT, EQ.
    /// Greater Than flag
    pub flag_gt: bool,
    /// Equal flag
    pub flag_eq: bool,
    pub current_frame: CallFrame,
    pub gas_left: Saturating<u32>,
}

impl Default for VMState {
    fn default() -> Self {}
}
// Arbitrary default, change it if you need to.
const DEFAULT_GAS_LIMIT: u32 = 1 << 16;
impl VMState {
    // TODO: The VM will probably not take the program to execute as a parameter later on.
    pub fn new(program_code: Vec<U256>) -> Self {
        Self {
            registers: [U256::zero(); 15],
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            current_frame: CallFrame::new(program_code, Rc::new(RefCell::new(InMemory::default()))),
            gas_left: Saturating(DEFAULT_GAS_LIMIT),
        }
    }
}

impl VMState {
    pub fn load_program(&mut self, program_code: Vec<U256>) -> &mut Self {
        self.current_frame.code_page = program_code;
        self
    }

    pub fn storage_path(&mut self, storage_path: String) -> &mut Self {
        self.current_frame.storage = Rc::new(RefCell::new(
            RocksDB::open(PathBuf::from(storage_path)).unwrap(),
        ));
        self
    }

    pub fn gas_left(&mut self, gas_limit: u32) -> &mut Self {
        self.gas_left = Saturating(gas_limit);
        self
    }

    pub fn flag_state(&mut self, flag_lt_of: bool, flag_eq: bool, flag_gt: bool) -> &mut Self {
        self.flag_lt_of = flag_lt_of;
        self.flag_eq = flag_eq;
        self.flag_gt = flag_gt;
        self
    }

    pub fn registers(&mut self, registers: [U256; 15]) -> &mut Self {
        self.registers = registers;
        self
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

    // This is redundant, but eventually this will have
    // some complex logic regarding the call frames,
    // so I'm future proofing it a little bit.
    pub fn gas_left(&self) -> u32 {
        self.gas_left.0
    }

    pub fn decrease_gas(&mut self, opcode: &Opcode) {
        self.gas_left -= opcode.variant.ergs_price();
    }
}

impl CallFrame {
    pub fn new(program_code: Vec<U256>, storage: Rc<RefCell<dyn Storage>>) -> Self {
        Self {
            stack: Stack::new(),
            heap: vec![],
            code_page: program_code,
            pc: 0,
            storage,
            transient_storage: InMemory::default(),
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

use std::collections::HashMap;
use std::num::Saturating;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::store::{GlobalStorage, RocksDB};
use crate::{
    opcode::Predicate,
    store::{InMemory, Storage},
    value::TaggedValue,
    Opcode,
};
use u256::{H160, U256};
use zkevm_opcode_defs::ethereum_types::Address;
use zkevm_opcode_defs::{OpcodeVariant, DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW};

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
    /// Transient storage should be used for temporary storage within a transaction and then discarded.
    pub transient_storage: InMemory,
    /// The address of the contract being executed in this frame.
    pub address: H160,
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
    pub current_frame: CallFrame,
    pub gas_left: u32,
}
impl Default for VMStateBuilder {
    fn default() -> Self {
        VMStateBuilder {
            registers: [TaggedValue::default(); 15],
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            current_frame: CallFrame::new(
                vec![],
                Rc::new(RefCell::new(InMemory::default())),
                H160::zero(),
            ),
            gas_left: DEFAULT_GAS_LIMIT,
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
    pub fn with_current_frame(mut self, frame: CallFrame) -> VMStateBuilder {
        self.current_frame = frame;
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
    pub fn gas_left(mut self, gas_left: u32) -> VMStateBuilder {
        self.gas_left = gas_left;
        self
    }
    pub fn with_contract_address(mut self, contract_address: H160) -> VMStateBuilder {
        self.current_frame.address = contract_address;
        self
    }
    pub fn with_storage(mut self, storage: PathBuf) -> VMStateBuilder {
        let storage = Rc::new(RefCell::new(RocksDB::open(storage).unwrap()));
        self.current_frame.storage = storage;
        self
    }
    pub fn build(self) -> VMState {
        VMState {
            registers: self.registers,
            current_frame: self.current_frame,
            flag_eq: self.flag_eq,
            flag_gt: self.flag_gt,
            flag_lt_of: self.flag_lt_of,
            gas_left: Saturating(self.gas_left),
            decommitted_hashes: HashMap::new(),
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
    pub current_frame: CallFrame,
    pub decommitted_hashes: HashMap<U256, ()>,
    pub gas_left: Saturating<u32>,
}

impl Default for VMState {
    fn default() -> Self {
        Self {
            registers: [TaggedValue::default(); 15],
            decommitted_hashes: HashMap::new(),
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            current_frame: CallFrame::new(
                vec![],
                Rc::new(RefCell::new(InMemory::default())),
                H160::zero(),
            ),
            gas_left: Saturating(DEFAULT_GAS_LIMIT),
        }
    }
}
// Arbitrary default, change it if you need to.
const DEFAULT_GAS_LIMIT: u32 = 1 << 16;
impl VMState {
    // TODO: The VM will probably not take the program to execute as a parameter later on.
    pub fn new(program_code: Vec<U256>) -> Self {
        Self {
            registers: [TaggedValue::default(); 15],
            flag_lt_of: false,
            flag_gt: false,
            flag_eq: false,
            current_frame: CallFrame::new(
                program_code,
                Rc::new(RefCell::new(InMemory::default())),
                H160::zero(),
            ),
            gas_left: Saturating(DEFAULT_GAS_LIMIT),
            decommitted_hashes: HashMap::new(),
        }
    }

    pub fn load_program(&mut self, program_code: Vec<U256>) {
        self.current_frame.code_page = program_code;
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
    pub fn new(program_code: Vec<U256>, storage: Rc<RefCell<dyn Storage>>, address: H160) -> Self {
        Self {
            address,
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

/// Used to load code when the VM is not yet initialized.
pub fn initial_decommit<T: GlobalStorage + Storage>(world_state: &mut T, address: H160) -> U256 {
    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let code_info = world_state
        .read(&(deployer_system_contract_address, address_into_u256(address)))
        .unwrap_or_default();

    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    code_info_bytes[1] = 0;
    let code_key: U256 = U256::from_big_endian(&code_info_bytes);

    world_state.decommit(&code_key)
}

/// Helper function to convert an H160 address into a U256.
/// Used to store the contract hash in the storage.
pub fn address_into_u256(address: H160) -> U256 {
    let mut buffer = [0; 32];
    buffer[12..].copy_from_slice(address.as_bytes());
    U256::from_big_endian(&buffer)
}

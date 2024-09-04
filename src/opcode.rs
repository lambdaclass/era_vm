use crate::eravm_error::EraVmError;
use crate::eravm_error::OpcodeError;
use lazy_static::lazy_static;
pub use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::OpcodeVariant;
use zkevm_opcode_defs::Operand;
use zkevm_opcode_defs::CONDITIONAL_BITS_SHIFT;
use zkevm_opcode_defs::DST_REGS_SHIFT;
use zkevm_opcode_defs::OPCODES_TABLE_WIDTH;
use zkevm_opcode_defs::SRC_REGS_SHIFT;
pub use zkevm_opcode_defs::{LogOpcode, RetOpcode, UMAOpcode};

#[derive(Debug, Clone)]
pub enum Predicate {
    Always = 0,
    Gt,
    Lt,
    Eq,
    Ge,
    Le,
    Ne,
    GtOrLt,
}

impl TryFrom<u8> for Predicate {
    type Error = EraVmError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = match value {
            x if x == Predicate::Always as u8 => Predicate::Always,
            x if x == Predicate::Gt as u8 => Predicate::Gt,
            x if x == Predicate::Lt as u8 => Predicate::Lt,
            x if x == Predicate::Eq as u8 => Predicate::Eq,
            x if x == Predicate::Ge as u8 => Predicate::Ge,
            x if x == Predicate::Le as u8 => Predicate::Le,
            x if x == Predicate::Ne as u8 => Predicate::Ne,
            x if x == Predicate::GtOrLt as u8 => Predicate::GtOrLt,
            _ => return Err(OpcodeError::InvalidPredicate.into()),
        };

        Ok(value)
    }
}

#[derive(Debug, Clone)]
pub struct Opcode {
    pub variant: Variant,
    pub src0_operand_type: Operand,
    pub dst0_operand_type: Operand,
    pub predicate: Predicate,
    pub flag0_set: bool,
    pub flag1_set: bool,
    pub src0_index: u8,
    pub src1_index: u8,
    pub dst0_index: u8,
    pub dst1_index: u8,
    pub imm0: u32,
    pub imm1: u32,
    pub gas_cost: u32,
}
lazy_static! {
    static ref OPCODE_TABLE: Vec<OpcodeVariant> =
        zkevm_opcode_defs::synthesize_opcode_decoding_tables(11, zkevm_opcode_defs::ISAVersion(2));
}

impl Opcode {
    const VARIANT_MASK: u64 = (1u64 << OPCODES_TABLE_WIDTH) - 1;

    pub fn try_from_raw_opcode_test_encode(raw_op: u128) -> Result<Self, EraVmError> {
        // First 11 bits
        let variant_bits = (raw_op as u64) & Self::VARIANT_MASK;
        let opcode_zksync = OPCODE_TABLE[variant_bits as usize];
        let [flag0_set, flag1_set] = match opcode_zksync.opcode {
            Variant::Ptr(_) => [false, opcode_zksync.flags[0]],
            _ => opcode_zksync.flags,
        };
        let predicate_byte: u8 = (((raw_op as u64) & 0xe000) >> CONDITIONAL_BITS_SHIFT) as u8;

        let imm0 = (raw_op >> 32) as u32;
        let imm1 = (raw_op >> 64) as u32;

        let gas_cost: u32 = opcode_zksync.opcode.ergs_price();

        let split_as_u4 = |value: u8| (value & ((1u8 << 4) - 1), value >> 4);

        let src_byte = (raw_op >> SRC_REGS_SHIFT) as u8;
        let dst_byte = (raw_op >> DST_REGS_SHIFT) as u8;

        let (src0_index, src1_index) = split_as_u4(src_byte);
        let (dst0_index, dst1_index) = split_as_u4(dst_byte);

        Ok(Self {
            variant: opcode_zksync.opcode,
            src0_operand_type: opcode_zksync.src0_operand_type,
            dst0_operand_type: opcode_zksync.dst0_operand_type,
            predicate: Predicate::try_from(predicate_byte)?,
            flag0_set,
            flag1_set,
            src0_index,
            src1_index,
            dst0_index,
            dst1_index,
            imm0,
            imm1,
            gas_cost,
        })
    }
    pub fn try_from_raw_opcode(raw_op: u64) -> Result<Self, EraVmError> {
        // First 11 bits
        let variant_bits = raw_op & 2047;
        let opcode_zksync = OPCODE_TABLE[variant_bits as usize];
        let [flag0_set, flag1_set] = match opcode_zksync.opcode {
            Variant::Ptr(_) => [false, opcode_zksync.flags[0]],
            _ => opcode_zksync.flags,
        };
        let predicate_u8: u8 = ((raw_op & 0xe000) >> 13) as u8;
        let src0_and_1_index: u8 = ((raw_op & 0xff0000) >> 16) as u8;
        let dst0_and_1_index: u8 = ((raw_op & 0xff000000) >> 24) as u8;

        let imm0 = ((raw_op & 0xffff00000000) >> 32) as u32;
        let imm1 = ((raw_op & 0xffff000000000000) >> 48) as u32;

        let gas_cost: u32 = opcode_zksync.opcode.ergs_price();

        let opcode = Self {
            variant: opcode_zksync.opcode,
            src0_operand_type: opcode_zksync.src0_operand_type,
            dst0_operand_type: opcode_zksync.dst0_operand_type,
            predicate: Predicate::try_from(predicate_u8)?,
            flag0_set,
            flag1_set,
            src0_index: first_four_bits(src0_and_1_index),
            src1_index: second_four_bits(src0_and_1_index),
            dst0_index: first_four_bits(dst0_and_1_index),
            dst1_index: second_four_bits(dst0_and_1_index),
            imm0,
            imm1,
            gas_cost,
        };

        Ok(opcode)
    }
}

#[inline(always)]
fn first_four_bits(value: u8) -> u8 {
    value & 0x0f
}

#[inline(always)]
fn second_four_bits(value: u8) -> u8 {
    (value & 0xf0) >> 4
}

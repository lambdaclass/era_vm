use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::Operand;

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

impl From<u8> for Predicate {
    fn from(value: u8) -> Self {
        match value {
            x if x == Predicate::Always as u8 => Predicate::Always,
            x if x == Predicate::Gt as u8 => Predicate::Gt,
            x if x == Predicate::Lt as u8 => Predicate::Lt,
            x if x == Predicate::Eq as u8 => Predicate::Eq,
            x if x == Predicate::Ge as u8 => Predicate::Ge,
            x if x == Predicate::Le as u8 => Predicate::Le,
            x if x == Predicate::Ne as u8 => Predicate::Ne,
            x if x == Predicate::GtOrLt as u8 => Predicate::GtOrLt,
            // TODO: maybe don't panic here
            _ => panic!("Unrecognized predicate: {}", value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Opcode {
    pub variant: Variant,
    pub src0_operand_type: Operand,
    pub dst0_operand_type: Operand,
    pub predicate: Predicate,
    // pub flags: [bool; 2],
    pub alters_vm_flags: bool,
    pub swap_flag: bool,
    pub src0_index: u8,
    pub src1_index: u8,
    pub dst0_index: u8,
    pub dst1_index: u8,
    pub imm0: u16,
    pub imm1: u16,
    pub gas_cost: u32,
}

impl Opcode {
    pub fn from_raw_opcode(raw_op: u64, opcode_table: &[zkevm_opcode_defs::OpcodeVariant]) -> Self {
        // First 11 bits
        let variant_bits = raw_op & 2047;
        let opcode_zksync = opcode_table[variant_bits as usize];
        let [alters_vm_flags, swap_flag] = opcode_zksync.flags;
        let predicate_u8: u8 = ((raw_op & 0xe000) >> 13) as u8;
        let src0_and_1_index: u8 = ((raw_op & 0xff0000) >> 16) as u8;
        let dst0_and_1_index: u8 = ((raw_op & 0xff000000) >> 24) as u8;

        let imm0: u16 = ((raw_op & 0xffff00000000) >> 32) as u16;
        let imm1: u16 = ((raw_op & 0xffff000000000000) >> 48) as u16;

        let gas_cost: u32 = opcode_zksync.opcode.ergs_price();

        Self {
            variant: opcode_zksync.opcode,
            src0_operand_type: opcode_zksync.src0_operand_type,
            dst0_operand_type: opcode_zksync.dst0_operand_type,
            predicate: Predicate::from(predicate_u8),
            alters_vm_flags,
            swap_flag,
            src0_index: first_four_bits(src0_and_1_index),
            src1_index: second_four_bits(src0_and_1_index),
            dst0_index: first_four_bits(dst0_and_1_index),
            dst1_index: second_four_bits(dst0_and_1_index),
            imm0,
            imm1,
            gas_cost,
        }
    }
}

fn first_four_bits(value: u8) -> u8 {
    value & 0x0f
}

fn second_four_bits(value: u8) -> u8 {
    (value & 0xf0) >> 4
}

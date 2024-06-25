use crate::address_operands::{address_operands_read, address_operands_store};
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};
use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::ShiftOpcode;

fn _shl(vm: &mut VMState, opcode: &Opcode) {
    let (src0_t, src1_t) = address_operands_read(vm, opcode);
    let (src0, src1) = (src0_t.value, src1_t.value);
    let shift = (src1.low_u32() % 256) as usize;
    let res = src0 << shift;
    if opcode.alters_vm_flags {
        // Eq is set if result == 0
        vm.flag_eq |= res.is_zero();
        // other flags are reset
        vm.flag_lt_of = false;
        vm.flag_gt = false;
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(res));
}

fn _shr(vm: &mut VMState, opcode: &Opcode) {
    let (src0_t, src1_t) = address_operands_read(vm, opcode);
    let (src0, src1) = (src0_t.value, src1_t.value);
    let shift = (src1.low_u32() % 256) as usize;
    let res = src0 >> shift;
    if opcode.alters_vm_flags {
        // Eq is set if result == 0
        vm.flag_eq |= res.is_zero();
        // other flags are reset
        vm.flag_lt_of = false;
        vm.flag_gt = false;
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(res));
}

fn _rol(vm: &mut VMState, opcode: &Opcode) {
    let (src0_t, src1_t) = address_operands_read(vm, opcode);
    let (src0, src1) = (src0_t.value, src1_t.value);
    let shift = (src1.low_u32() % 256) as usize;
    let result = (src0 << shift) | (src0 >> (256 - shift));
    if opcode.alters_vm_flags {
        // Eq is set if result == 0
        vm.flag_eq |= result.is_zero();
        // other flags are reset
        vm.flag_lt_of = false;
        vm.flag_gt = false;
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(result));
}

fn _ror(vm: &mut VMState, opcode: &Opcode) {
    let (src0_t, src1_t) = address_operands_read(vm, opcode);
    let (src0, src1) = (src0_t.value, src1_t.value);
    let shift = (src1.low_u32() % 256) as usize;
    let result = (src0 >> shift) | (src0 << (256 - shift));
    if opcode.alters_vm_flags {
        // Eq is set if result == 0
        vm.flag_eq |= result.is_zero();
        // other flags are reset
        vm.flag_lt_of = false;
        vm.flag_gt = false;
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(result));
}

pub fn _shift(vm: &mut VMState, opcode: &Opcode) {
    match opcode.variant {
        Variant::Shift(ShiftOpcode::Shl) => _shl(vm, opcode),
        Variant::Shift(ShiftOpcode::Shr) => _shr(vm, opcode),
        Variant::Shift(ShiftOpcode::Rol) => _rol(vm, opcode),
        Variant::Shift(ShiftOpcode::Ror) => _ror(vm, opcode),
        _ => panic!("Unsupported shift operation"),
    }
}

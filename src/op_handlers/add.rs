use crate::{opcode::Opcode, state::VMState};
use zkevm_opcode_defs::Operand;
use zkevm_opcode_defs::ImmMemHandlerFlags;
use zkevm_opcode_defs::RegOrImmFlags;
use u256::U256;



fn _add_reg_only(vm: &mut VMState, opcode: Opcode) {
    // src0 + src1 -> dst0
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    vm.set_register(opcode.dst0_index, src0 + src1);
}

fn _add_imm16_only(vm: &mut VMState, opcode: Opcode) {
    // imm0 + src0 -> dst0
    let src1 = vm.get_register(opcode.src1_index);
    vm.set_register(opcode.dst0_index, src1 + opcode.imm0);
}

pub fn _add(vm: &mut VMState, opcode: Opcode) {
    match opcode.src0_operand_type {
        Operand::RegOnly => {
            _add_reg_only(vm, opcode);
        },
        Operand::RegOrImm(variant) => {
            match variant {
                RegOrImmFlags::UseRegOnly => {
                    _add_reg_only(vm, opcode);
                },
                RegOrImmFlags::UseImm16Only => {
                    _add_imm16_only(vm, opcode);
                },
            }
        },
        Operand::Full(variant) => {
            match variant {
                ImmMemHandlerFlags::UseRegOnly => {
                    _add_reg_only(vm, opcode);
                },
                ImmMemHandlerFlags::UseStackWithPushPop => {
                    // stack-=[src0 + offset] + src1 -> dst0
                    panic!("Add with UseStackWithPushPop is not allowed")
                },
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1 -> dst0
                    let src0 = vm.get_register(opcode.src0_index);
                    let src1 = vm.get_register(opcode.src1_index);
                    let offset = opcode.imm0; 

                    let sp = vm.current_frame.stack.len();
                    let res = vm.current_frame.stack[sp - (src0 + U256::from(offset)).as_usize()].value.clone();
                    vm.set_register(opcode.dst0_index, res + src1);
                },
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1 -> dst0
                    let src0 = vm.get_register(opcode.src0_index);
                    let src1 = vm.get_register(opcode.src1_index);
                    let offset = opcode.imm0; 

                    let res = vm.current_frame.stack[(src0 + U256::from(offset)).as_usize()].value.clone();
                    vm.set_register(opcode.dst0_index, res + src1);
                },
                ImmMemHandlerFlags::UseImm16Only => {
                    _add_imm16_only(vm, opcode);
                },
                ImmMemHandlerFlags::UseCodePage => {
                    let src0 = vm.get_register(opcode.src0_index);
                    let src1 = vm.get_register(opcode.src1_index);
                    let offset = opcode.imm0; 

                    let res = vm.current_frame.code_page[(src0 + U256::from(offset)).as_usize()];
                    vm.set_register(opcode.dst0_index, res + src1);
                },
            }
        },
    }
}

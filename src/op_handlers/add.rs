use crate::{opcode::Opcode, state::VMState};
use zkevm_opcode_defs::ImmMemHandlerFlags;
use zkevm_opcode_defs::Operand;

pub fn _add(vm: &mut VMState, opcode: Opcode) {
    match opcode.src0_operand_type {
        Operand::RegOnly => todo!(),
        Operand::RegOrImm(_) => todo!(),
        Operand::Full(variant) => {
            match variant {
                ImmMemHandlerFlags::UseRegOnly => {
                    // src0 + src1 -> dst0
                    let src0 = vm.get_register(opcode.src0_index);
                    let src1 = vm.get_register(opcode.src1_index);
                    vm.set_register(opcode.dst0_index, src0 + src1);
                }
                ImmMemHandlerFlags::UseStackWithPushPop => todo!(),
                ImmMemHandlerFlags::UseStackWithOffset => todo!(),
                ImmMemHandlerFlags::UseAbsoluteOnStack => todo!(),
                ImmMemHandlerFlags::UseImm16Only => {
                    // imm0 + src0 -> dst0
                    let src0 = vm.get_register(opcode.src0_index);
                    vm.set_register(opcode.dst0_index, src0 + opcode.imm0);
                }
                ImmMemHandlerFlags::UseCodePage => todo!(),
            }
        }
    }
}

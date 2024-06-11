use u256::U256;
use zkevm_opcode_defs::{ImmMemHandlerFlags, Operand, RegOrImmFlags};

use crate::{state::VMState, value::TaggedValue, Opcode};

fn only_reg_read(vm: &mut VMState, opcode: &Opcode) -> (U256, U256) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    return (src0, src1);
}

fn only_imm16_read(vm: &mut VMState, opcode: &Opcode) -> (U256, U256) {
    let src1 = vm.get_register(opcode.src1_index);
    return (U256::from(opcode.imm0), src1);
}

fn reg_and_imm_read(vm: &mut VMState, opcode: &Opcode) -> (U256, U256) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    let offset = opcode.imm0;

    return (src0 + U256::from(offset), src1);
}

pub fn address_operands_read(vm: &mut VMState, opcode: &Opcode) -> (U256, U256) {
    match opcode.src0_operand_type {
        Operand::RegOnly => {
            return only_reg_read(vm, opcode);
        }
        Operand::RegOrImm(variant) => match variant {
            RegOrImmFlags::UseRegOnly => {
                return only_reg_read(vm, opcode);
            }
            RegOrImmFlags::UseImm16Only => {
                return only_imm16_read(vm, opcode);
            }
        },
        Operand::Full(variant) => {
            match variant {
                ImmMemHandlerFlags::UseRegOnly => {
                    return only_reg_read(vm, opcode);
                }
                ImmMemHandlerFlags::UseStackWithPushPop => {
                    // stack-=[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);
                    let res = vm.current_frame.stack[vm.sp() - src0.as_usize()]
                        .value
                        .clone();
                    // TODO: Add push/pop stack
                    return (res, src1);
                }
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);

                    let res = vm.current_frame.stack[vm.sp() - src0.as_usize()]
                        .value
                        .clone();
                    return (res, src1);
                }
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);
                    let res = vm.current_frame.stack[src0.as_usize()].value.clone();
                    return (res, src1);
                }
                ImmMemHandlerFlags::UseImm16Only => {
                    return only_imm16_read(vm, opcode);
                }
                ImmMemHandlerFlags::UseCodePage => {
                    let (src0, src1) = reg_and_imm_read(vm, opcode);

                    let res = vm.current_frame.code_page[src0.as_usize()];
                    return (res, src1);
                }
            }
        }
    }
}
fn only_reg_write(vm: &mut VMState, opcode: &Opcode, res: U256) {
    vm.set_register(opcode.dst0_index, res);
}

fn reg_and_imm_write(vm: &mut VMState, opcode: &Opcode) -> U256 {
    let dst0 = vm.get_register(opcode.dst0_index);
    let offset = opcode.imm0;

    return dst0 + U256::from(offset);
}

pub fn address_operands_store(vm: &mut VMState, opcode: &Opcode, res: U256) {
    match opcode.dst0_operand_type {
        Operand::RegOnly => {
            only_reg_write(vm, opcode, res);
        }
        Operand::RegOrImm(variant) => match variant {
            RegOrImmFlags::UseRegOnly => {
                only_reg_write(vm, opcode, res);
            }
            RegOrImmFlags::UseImm16Only => {
                panic!("dest cannot be imm16 only");
            }
        },
        Operand::Full(variant) => {
            match variant {
                ImmMemHandlerFlags::UseRegOnly => {
                    only_reg_write(vm, opcode, res);
                }
                ImmMemHandlerFlags::UseStackWithPushPop => {
                    // stack-=[src0 + offset] + src1
                    let src0 = reg_and_imm_write(vm, opcode);
                    let sp = vm.sp();
                    vm.current_frame.stack[sp - src0.as_usize()] = TaggedValue {
                        value: res,
                        is_pointer: false,
                    };
                    // TODO: Add push/pop stack
                }
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1
                    let src0 = reg_and_imm_write(vm, opcode);
                    let sp = vm.sp();
                    vm.current_frame.stack[sp - src0.as_usize()] = TaggedValue {
                        value: res,
                        is_pointer: false,
                    };
                }
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1
                    let src0 = reg_and_imm_write(vm, opcode);
                    vm.current_frame.stack[src0.as_usize()] = TaggedValue {
                        value: res,
                        is_pointer: false,
                    };
                }
                ImmMemHandlerFlags::UseImm16Only => {
                    panic!("dest cannot be imm16 only");
                }
                ImmMemHandlerFlags::UseCodePage => {
                    panic!("dest cannot be code page");
                }
            }
        }
    }
}

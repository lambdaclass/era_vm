use u256::U256;
use zkevm_opcode_defs::{ImmMemHandlerFlags, Operand, RegOrImmFlags};

use crate::{
    eravm_error::{EraVmError, OperandError},
    state::VMState,
    value::TaggedValue,
    Opcode,
};

fn only_reg_read(vm: &VMState, opcode: &Opcode) -> (TaggedValue, TaggedValue) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    (src0, src1)
}

fn only_imm16_read(vm: &VMState, opcode: &Opcode) -> (TaggedValue, TaggedValue) {
    let src1 = vm.get_register(opcode.src1_index);
    (TaggedValue::new_raw_integer(U256::from(opcode.imm0)), src1)
}

fn reg_and_imm_read(vm: &VMState, opcode: &Opcode) -> (TaggedValue, TaggedValue) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    let offset = opcode.imm0;

    (
        src0 + TaggedValue::new_raw_integer(U256::from(offset)),
        src1,
    )
}

pub fn address_operands_read(
    vm: &mut VMState,
    opcode: &Opcode,
) -> Result<(TaggedValue, TaggedValue), EraVmError> {
    let (op1, op2) = match opcode.src0_operand_type {
        Operand::RegOnly => only_reg_read(vm, opcode),
        Operand::RegOrImm(variant) => match variant {
            RegOrImmFlags::UseRegOnly => only_reg_read(vm, opcode),
            RegOrImmFlags::UseImm16Only => only_imm16_read(vm, opcode),
        },
        Operand::Full(variant) => {
            match variant {
                ImmMemHandlerFlags::UseRegOnly => only_reg_read(vm, opcode),
                ImmMemHandlerFlags::UseStackWithPushPop => {
                    // stack-=[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);
                    let sp = vm.current_frame()?.sp;
                    let res = vm
                        .current_context()?
                        .stack
                        .get_with_offset(src0.value.as_usize(), sp)?;
                    vm.current_frame_mut()?.sp -= src0.value.low_u64();
                    (res, src1)
                }
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);
                    let sp = vm.current_frame()?.sp;
                    let res = vm
                        .current_context()?
                        .stack
                        .get_with_offset(src0.value.as_usize(), sp)?;

                    (res, src1)
                }
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);
                    let sp = vm.current_frame()?.sp;
                    let res = vm
                        .current_context()?
                        .stack
                        .get_absolute(src0.value.as_usize(), sp)?;

                    (res, src1)
                }
                ImmMemHandlerFlags::UseImm16Only => only_imm16_read(vm, opcode),
                ImmMemHandlerFlags::UseCodePage => {
                    let (src0, src1) = reg_and_imm_read(vm, opcode);

                    let res = vm.current_frame()?.code_page[src0.value.as_usize()];
                    (TaggedValue::new_raw_integer(res), src1)
                }
            }
        }
    };
    if opcode.swap_flag {
        Ok((op2, op1))
    } else {
        Ok((op1, op2))
    }
}

/// The first operand is used most of the times
/// the second operand is used only for div and mul
enum OutputOperandPosition {
    First,
    Second,
}

fn only_reg_write(
    vm: &mut VMState,
    opcode: &Opcode,
    output_op_pos: OutputOperandPosition,
    res: TaggedValue,
) {
    match output_op_pos {
        OutputOperandPosition::First => vm.set_register(opcode.dst0_index, res),
        OutputOperandPosition::Second => vm.set_register(opcode.dst1_index, res),
    }
}

fn reg_and_imm_write(
    vm: &mut VMState,
    output_op_pos: OutputOperandPosition,
    opcode: &Opcode,
) -> TaggedValue {
    match output_op_pos {
        OutputOperandPosition::First => {
            let dst0 = vm.get_register(opcode.dst0_index);
            let offset = opcode.imm1;
            let res = dst0 + TaggedValue::new_raw_integer(U256::from(offset));
            vm.set_register(opcode.dst0_index, res);
            res
        }
        OutputOperandPosition::Second => {
            let dst1 = vm.get_register(opcode.dst1_index);
            let offset = opcode.imm1;
            let res = dst1 + TaggedValue::new_raw_integer(U256::from(offset));
            vm.set_register(opcode.dst1_index, res);
            res
        }
    }
}

pub fn address_operands_store(
    vm: &mut VMState,
    opcode: &Opcode,
    res: TaggedValue,
) -> Result<(), EraVmError> {
    address_operands(vm, opcode, (res, None))
}

pub fn address_operands_div_mul(
    vm: &mut VMState,
    opcode: &Opcode,
    res: (TaggedValue, TaggedValue),
) -> Result<(), EraVmError> {
    address_operands(vm, opcode, (res.0, Some(res.1)))
}

fn address_operands(
    vm: &mut VMState,
    opcode: &Opcode,
    res: (TaggedValue, Option<TaggedValue>),
) -> Result<(), EraVmError> {
    match opcode.dst0_operand_type {
        Operand::RegOnly => {
            only_reg_write(vm, opcode, OutputOperandPosition::First, res.0);
        }
        Operand::RegOrImm(variant) => match variant {
            RegOrImmFlags::UseRegOnly => {
                only_reg_write(vm, opcode, OutputOperandPosition::First, res.0);
            }
            RegOrImmFlags::UseImm16Only => {
                return Err(OperandError::InvalidDestImm16Only(opcode.variant).into());
            }
        },
        Operand::Full(variant) => {
            match variant {
                ImmMemHandlerFlags::UseRegOnly => {
                    only_reg_write(vm, opcode, OutputOperandPosition::First, res.0);
                }
                ImmMemHandlerFlags::UseStackWithPushPop => {
                    // stack+=[src0 + offset] + src1
                    let src0 = reg_and_imm_write(vm, OutputOperandPosition::First, opcode);
                    vm.current_frame_mut()?.sp += src0.value.low_u64();
                }
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1
                    let src0 = reg_and_imm_write(vm, OutputOperandPosition::First, opcode);
                    let sp = vm.current_frame()?.sp;
                    vm.current_context_mut()?.stack.store_with_offset(
                        src0.value.as_usize(),
                        res.0,
                        sp,
                    )?;
                }
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1
                    let src0 = reg_and_imm_write(vm, OutputOperandPosition::First, opcode);
                    let sp = vm.current_frame()?.sp;
                    vm.current_context_mut()?.stack.store_absolute(
                        src0.value.as_usize(),
                        res.0,
                        sp,
                    )?;
                }
                ImmMemHandlerFlags::UseImm16Only => {
                    return Err(OperandError::InvalidDestImm16Only(opcode.variant).into());
                }
                ImmMemHandlerFlags::UseCodePage => {
                    return Err(OperandError::InvalidDestCodePage(opcode.variant).into());
                }
            }
        }
    }
    if let Some(res) = res.1 {
        // Second operand can only be a register
        only_reg_write(vm, opcode, OutputOperandPosition::Second, res);
    };
    Ok(())
}

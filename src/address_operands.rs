use u256::U256;
use zkevm_opcode_defs::{ImmMemHandlerFlags, Operand, RegOrImmFlags};

use crate::{
    eravm_error::{EraVmError, OperandError},
    execution::Execution,
    utils::LowUnsigned,
    value::TaggedValue,
    Opcode,
};

fn only_reg_read(vm: &Execution, opcode: &Opcode) -> (TaggedValue, TaggedValue) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    (src0, src1)
}

fn only_imm16_read(vm: &Execution, opcode: &Opcode) -> (TaggedValue, TaggedValue) {
    let src1 = vm.get_register(opcode.src1_index);
    (TaggedValue::new_raw_integer(U256::from(opcode.imm0)), src1)
}

fn reg_and_imm_read(vm: &Execution, opcode: &Opcode) -> (TaggedValue, TaggedValue) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    let offset = opcode.imm0;

    (
        src0 + TaggedValue::new_raw_integer(U256::from(offset)),
        src1,
    )
}

pub fn address_operands_read(
    vm: &mut Execution,
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
                        .get_with_offset(src0.value.low_u16(), sp)?;
                    vm.current_frame_mut()?.sp -= src0.value.low_u16() as u32;
                    (res, src1)
                }
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);
                    let sp = vm.current_frame()?.sp;
                    let res = vm
                        .current_context()?
                        .stack
                        .get_with_offset(src0.value.low_u16(), sp)?;

                    (res, src1)
                }
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1
                    let (src0, src1) = reg_and_imm_read(vm, opcode);
                    let sp = vm.current_frame()?.sp;
                    let res = vm
                        .current_context()?
                        .stack
                        .get_absolute(src0.value.low_u16(), sp)?;

                    (res, src1)
                }
                ImmMemHandlerFlags::UseImm16Only => only_imm16_read(vm, opcode),
                ImmMemHandlerFlags::UseCodePage => {
                    let (src0, src1) = reg_and_imm_read(vm, opcode);

                    let res = vm
                        .current_context()?
                        .code_page
                        .get(src0.value.low_u16() as usize);
                    (TaggedValue::new_raw_integer(res), src1)
                }
            }
        }
    };
    if opcode.flag1_set {
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
    vm: &mut Execution,
    opcode: &Opcode,
    output_op_pos: OutputOperandPosition,
    res: TaggedValue,
) {
    match output_op_pos {
        OutputOperandPosition::First => vm.set_register(opcode.dst0_index, res),
        OutputOperandPosition::Second => vm.set_register(opcode.dst1_index, res),
    }
}

fn dest_stack_address(vm: &mut Execution, opcode: &Opcode) -> TaggedValue {
    let dst0 = vm.get_register(opcode.dst0_index);
    let offset = opcode.imm1;
    dst0 + TaggedValue::new_raw_integer(U256::from(offset))
}

pub fn address_operands_store(
    vm: &mut Execution,
    opcode: &Opcode,
    res: TaggedValue,
) -> Result<(), EraVmError> {
    address_operands(vm, opcode, (res, None))
}

pub fn address_operands_div_mul(
    vm: &mut Execution,
    opcode: &Opcode,
    res: (TaggedValue, TaggedValue),
) -> Result<(), EraVmError> {
    address_operands(vm, opcode, (res.0, Some(res.1)))
}

fn address_operands(
    vm: &mut Execution,
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
                    let src0 = dest_stack_address(vm, opcode);
                    vm.current_frame_mut()?.sp += src0.value.low_u16() as u32;
                }
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1
                    let src0 = dest_stack_address(vm, opcode);
                    let sp = vm.current_frame()?.sp;
                    vm.current_context_mut()?.stack.store_with_offset(
                        src0.value.low_u16(),
                        res.0,
                        sp,
                    )?;
                }
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1
                    let src0 = dest_stack_address(vm, opcode);
                    vm.current_context_mut()?
                        .stack
                        .store_absolute(src0.value.low_u16(), res.0)?;
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

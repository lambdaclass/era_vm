fn only_reg(vm: &mut VMState, opcode: Opcode) -> (U256, U256) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    return (src0, src1);
}

fn only_imm16(vm: &mut VMState, opcode: Opcode) -> (U256, U256) {
    let src1 = vm.get_register(opcode.src1_index);
    return (U256::from(opcode.imm0), src1);
}

fn stack(vm: &mut VMState, opcode: Opcode) -> (U256, U256) {
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    let offset = opcode.imm0; 

    return (src0 + U256::from(offset), src1);
}

pub fn address_operands_read(vm: &mut VMState, opcode: Opcode) -> (U256, U256) {
    match opcode.src0_operand_type {
        Operand::RegOnly => {
            return only_reg(vm, opcode);
        },
        Operand::RegOrImm(variant) => {
            match variant {
                RegOrImmFlags::UseRegOnly => {
                    return only_reg(vm, opcode);
                },
                RegOrImmFlags::UseImm16Only => {
                    return only_imm16(vm, opcode);
                },
            }
        },
        Operand::Full(variant) => {
            match variant {
                ImmMemHandlerFlags::UseRegOnly => {
                    return only_reg(vm, opcode);
                },
                ImmMemHandlerFlags::UseStackWithPushPop => {
                    // stack-=[src0 + offset] + src1
                    
                },
                ImmMemHandlerFlags::UseStackWithOffset => {
                    // stack[src0 + offset] + src1
                    let (src0, src1) = stack(vm, opcode);

                    let res = vm.current_frame.stack[vm.sp() - src0.as_usize()].value.clone();
                    return (res, src1);
                },
                ImmMemHandlerFlags::UseAbsoluteOnStack => {
                    // stack=[src0 + offset] + src1
                    let (src0, src1) = stack(vm, opcode);
                    let res = vm.current_frame.stack[src0.as_usize()].value.clone();
                    return (res, src1);
                },
                ImmMemHandlerFlags::UseImm16Only => {
                    return only_imm16(vm, opcode);
                },
                ImmMemHandlerFlags::UseCodePage => {
                },
            }
        },
    }
}

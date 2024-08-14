use zkevm_opcode_defs::InvalidOpcode;

use crate::{eravm_error::EraVmError, state::VMState, Opcode};

pub fn debug_instr(
    vm: &mut VMState,
    opcode: &Opcode,
    i: &mut u64,
    only_instr: bool,
    nop_if_not_predicate: bool,
    print_registers: bool,
    show_flags: bool,
) -> Result<(), EraVmError> {
    *i += 1;

    let variant = if nop_if_not_predicate && !vm.can_execute(opcode)? {
        zkevm_opcode_defs::Opcode::Invalid(InvalidOpcode)
    } else {
        opcode.variant
    };
    println!("{} - INSTR: {:?}", i, variant);
    if !only_instr {
        println!("PC: {}", vm.current_frame()?.pc);
        println!("GAS LEFT: {}", vm.current_frame()?.gas_left);
        println!("SP: {}", vm.current_frame()?.sp);
        // println!("PREDICATE: {:?}", opcode.predicate);

        if show_flags {
            println!(
                "GT: {}, LT_OF: {}, EQ: {}",
                vm.flag_gt, vm.flag_lt_of, vm.flag_eq
            );
        }
    }

    if print_registers {
        for i in 0..16 {
            let reg = vm.get_register(i);
            println!("REG: {} VAL: {} IS_PTR {}", i, reg.value, reg.is_pointer);
        }
    };

    Ok(())
}

// pub fn read_from_pointer(&self, pointer: &FatPointer) -> U256 {
//     let start: u32 = pointer.start + pointer.offset.min(pointer.len);
//     let end = start.saturating_add(32).min(pointer.start + pointer.len);
//     let mut result: [u8; 32] = [0; 32];
//     for i in 0..32 {
//         let addr = start + i;
//         if addr < end {
//             result[i as usize] = self.heap[addr as usize];
//         } else {
//             result[i as usize] = 0;
//         }
//     }
//     U256::from_big_endian(&result)
// }

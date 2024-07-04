use u256::U256;
use zkevm_opcode_defs::Opcode as ZKOpcode;
use zkevm_opcode_defs::UMAOpcode;

use crate::address_operands::address_operands_read;
use crate::value::FatPointer;
use crate::{state::VMState, Opcode};

use super::tracer::Tracer;

pub struct PrintTracer {}

impl Tracer for PrintTracer {
    #[allow(clippy::println_empty_string)]
    fn before_execution(&mut self, opcode: &Opcode, vm: &VMState) {
        let opcode_variant = opcode.variant;

        const DEBUG_SLOT: u32 = 1024;

        let debug_magic = U256::from_dec_str(
            "33509158800074003487174289148292687789659295220513886355337449724907776218753",
        )
        .unwrap();

        if matches!(opcode_variant, ZKOpcode::UMA(UMAOpcode::HeapWrite)) {
            let mut new_vm = vm.clone();
            let (src0, src1) = address_operands_read(&mut new_vm, opcode).unwrap();
            let value = src1.value;
            if value == debug_magic {
                let fat_ptr = FatPointer::decode(src0.value);
                if fat_ptr.offset == DEBUG_SLOT {
                    let how_to_print_value = new_vm
                        .current_frame_mut()
                        .unwrap()
                        .heap
                        .read(DEBUG_SLOT + 32);
                    let value_to_print = new_vm
                        .current_frame_mut()
                        .unwrap()
                        .heap
                        .read(DEBUG_SLOT + 64);

                    let print_as_hex_value = U256::from_str_radix(
                        "0x00debdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebde",
                        16,
                    )
                    .unwrap();
                    let print_as_string_value = U256::from_str_radix(
                        "0x00debdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdf",
                        16,
                    )
                    .unwrap();

                    if how_to_print_value == print_as_hex_value {
                        print!("PRINTED: ");
                        println!("0x{:02x}", value_to_print);
                    }

                    if how_to_print_value == print_as_string_value {
                        print!("PRINTED: ");
                        let mut value = value_to_print.0;
                        value.reverse();
                        for limb in value {
                            print!(
                                "{}",
                                String::from_utf8(limb.to_be_bytes().to_vec()).unwrap()
                            );
                        }
                        println!("");
                    }
                }
            }
        }
    }
}

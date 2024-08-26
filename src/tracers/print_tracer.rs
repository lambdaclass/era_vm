use u256::U256;
use zkevm_opcode_defs::Opcode as ZKOpcode;
use zkevm_opcode_defs::UMAOpcode;

use crate::address_operands::address_operands_read;
use crate::state::VMState;
use crate::value::FatPointer;
use crate::{execution::Execution, Opcode};

use super::tracer::Tracer;

pub struct PrintTracer {}

impl Tracer for PrintTracer {
    #[allow(clippy::println_empty_string)]
    fn before_execution(&mut self, opcode: &Opcode, vm: &mut Execution, _state: &mut VMState) {
        let opcode_variant = opcode.variant;

        const DEBUG_SLOT: u32 = 1024;

        let Ok(debug_magic) = U256::from_dec_str(
            "33509158800074003487174289148292687789659295220513886355337449724907776218753",
        ) else {
            return;
        };

        if matches!(opcode_variant, ZKOpcode::UMA(UMAOpcode::HeapWrite)) {
            let Ok((src0, src1)) = address_operands_read(vm, opcode) else {
                return;
            };
            let value = src1.value;
            if value == debug_magic {
                let fat_ptr = FatPointer::decode(src0.value);
                if fat_ptr.offset == DEBUG_SLOT {
                    let Ok(ctx) = vm.current_context() else {
                        return;
                    };
                    let Some(heap) = vm.heaps.get(ctx.heap_id) else {
                        return;
                    };
                    let how_to_print_value = heap.read(DEBUG_SLOT + 32);
                    let value_to_print = heap.read(DEBUG_SLOT + 64);

                    let Ok(print_as_hex_value) = U256::from_str_radix(
                        "0x00debdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebde",
                        16,
                    ) else {
                        return;
                    };

                    let Ok(print_as_string_value) = U256::from_str_radix(
                        "0x00debdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdf",
                        16,
                    ) else {
                        return;
                    };

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

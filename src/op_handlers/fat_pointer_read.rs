use crate::address_operands::address_operands_read;
use crate::value::{FatPointer, TaggedValue};
use crate::{opcode::Opcode, state::VMState};

pub fn fat_pointer_read(vm: &mut VMState, opcode: &Opcode) {
    let (src0, _) = address_operands_read(vm, opcode);
    if !src0.is_pointer {
        panic!("Invalid operands for fat_pointer_read");
    }
    let pointer = FatPointer::decode(src0.value);

    if pointer.offset < pointer.len {
        let value = vm.current_frame_mut().heap.read_from_pointer(&pointer);

        vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));

        if opcode.alters_vm_flags {
            // This flag is set if .inc is present
            let new_pointer = FatPointer {
                offset: pointer.offset + 32,
                ..pointer
            };

            vm.set_register(
                opcode.dst1_index,
                TaggedValue::new_pointer(new_pointer.encode()),
            );
        }
    }
}

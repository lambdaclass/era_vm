use crate::{
    state::{Event, VMState},
    Opcode,
};

pub fn event(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index).value;
    let value = vm.get_register(opcode.src1_index).value;
    let event = Event {
        key,
        value,
        is_first: opcode.alters_vm_flags,
        shard_id: 1, // TODO: Shard Ids are not yet implemented
        tx_number: vm.tx_number as u16,
    };

    vm.events.push(event);
}

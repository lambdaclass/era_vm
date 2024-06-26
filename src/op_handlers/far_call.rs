use std::{cell::RefCell, collections::HashMap, rc::Rc};

use zkevm_opcode_defs::{abi::far_call, FarCallOpcode};

use crate::{
    address_operands::address_operands_read, state::VMState, store::InMemory, value::FatPointer,
    Opcode,
};
//   Inductive fwd_memory :=
//      ForwardFatPointer (p:fat_ptr)
//    | ForwardNewFatPointer (heap_var: data_page_type) (s:span).
//   Record params :=
//     mk_params {
//         fwd_memory: fwd_memory;
//         ergs_passed: ergs;
//         shard_id: shard_id;
//         constructor_call: bool;
//         to_system: bool;
// }.
struct FarCallParams {
    forward_memory: FatPointer,
    /// Gas stipend: how much gas the called contract
    /// will have for the call.
    ergs_passed: u32,
    shard_id: u8,
    constructor_call: bool,
    to_system: bool,
}
// TODO: Far call must
// 1 - Decode the parameters.
// 2 - Decommit the address.
// 3 - Load the new context.
pub fn far_call(vm: &mut VMState, opcode: &Opcode, far_call: &FarCallOpcode) {
    let (src0, src1) = address_operands_read(vm, opcode);
    dbg!(src1);
    let ergs_passed = src0.value.0[3] as u32;
    let _err_routine = opcode.imm0;
    let mut args = [0u8; 32];
    src0.value.to_little_endian(&mut args);
    let [.., shard_id, constructor_call_byte, system_call_byte] = args;
    let params = FarCallParams {
        forward_memory: FatPointer::decode(src0.value),
        ergs_passed,
        constructor_call: constructor_call_byte != 0,
        to_system: system_call_byte != 0,
        shard_id,
    };
    match far_call {
        FarCallOpcode::Normal => {
            let program_code = vm.current_context().code_page.clone();
            let stipend = vm.current_context().gas_left;
            let storage = Rc::new(RefCell::new(InMemory(HashMap::new())));
            vm.push_frame(program_code, stipend.0 / 32, storage)
        }
        _ => todo!(),
    }
}
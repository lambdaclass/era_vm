use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Sub, SubAssign},
    rc::Rc,
};

use u256::{H160, U256};
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
fn address_from_u256(register_value: &U256) -> H160 {
    let mut buffer: [u8; 32] = [0; 32];
    register_value.to_big_endian(&mut buffer[..]);
    H160::from_slice(&buffer[0..19])
}
// TODO: Far call must
// 1 - Decode the parameters. (done)
// 2 - Decommit the address.
// 3 - Load the new context.
pub fn far_call(vm: &mut VMState, opcode: &Opcode, far_call: &FarCallOpcode) {
    let (src0, src1) = address_operands_read(vm, opcode);
    dbg!(src1);
    let contract_address = address_from_u256(&src0.value);
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
        FarCallOpcode::Normal if ergs_passed < vm.current_context().gas_left.0 => {
            let program_code = vm.decommit_from_address(&contract_address);
            vm.current_context_mut().gas_left -= ergs_passed;
            vm.push_frame(program_code, ergs_passed, contract_address)
        }
        _ => todo!(),
    }
}

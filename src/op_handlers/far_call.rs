use u256::{H160, U256};
use zkevm_opcode_defs::FarCallOpcode;

use crate::{address_operands::address_operands_read, state::VMState, value::FatPointer, Opcode};
#[allow(dead_code)]
struct FarCallParams {
    forward_memory: FatPointer,
    /// Gas stipend: how much gas the called contract
    /// will have for the call.
    ergs_passed: u32,
    shard_id: u8,
    constructor_call: bool,
    /// If the far call is in kernel mode.
    to_system: bool,
}
fn far_call_params_from_register(source: U256) -> FarCallParams {
    let mut args = [0u8; 32];
    let ergs_passed = source.0[3] as u32;
    source.to_little_endian(&mut args);
    let [.., shard_id, constructor_call_byte, system_call_byte] = args;
    FarCallParams {
        forward_memory: FatPointer::decode(source),
        ergs_passed,
        constructor_call: constructor_call_byte != 0,
        to_system: system_call_byte != 0,
        shard_id,
    }
}
fn address_from_u256(register_value: &U256) -> H160 {
    let mut buffer: [u8; 32] = [0; 32];
    register_value.to_big_endian(&mut buffer[..]);
    H160::from_slice(&buffer[0..20])
}
// TODO: Far call must
// 1 - Decode the parameters. (done)
// 2 - Decommit the address. (WIP)
// 3 - Load the new context. (WIP)
pub fn far_call(vm: &mut VMState, opcode: &Opcode, far_call: &FarCallOpcode) {
    let (src0, src1) = address_operands_read(vm, opcode);
    let contract_address = address_from_u256(&src1.value);
    let _err_routine = opcode.imm0;
    let FarCallParams { ergs_passed, .. } = far_call_params_from_register(src0.value);
    match far_call {
        FarCallOpcode::Normal if ergs_passed < vm.current_context().gas_left.0 => {
            let program_code = vm.decommit_from_address(&contract_address);
            vm.current_context_mut().gas_left -= ergs_passed;
            vm.push_frame(program_code, ergs_passed, contract_address)
        }
        _ => todo!(),
    }
}

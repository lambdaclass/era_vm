use u256::{H160, U256};
use zkevm_opcode_defs::FarCallOpcode;

use crate::{
    address_operands::address_operands_read, state::VMState, store::Storage,
    utils::address_into_u256, value::FatPointer, Opcode,
};
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
const FAR_CALL_GAS_SCALAR_MODIFIER: u32 = 63 / 64;
fn far_call_params_from_register(source: U256, gas_left: u32) -> FarCallParams {
    let mut args = [0u8; 32];
    let mut ergs_passed = source.0[3] as u32;
    if ergs_passed > gas_left {
        ergs_passed = gas_left * (FAR_CALL_GAS_SCALAR_MODIFIER);
    }
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
    H160::from_slice(&buffer[12..])
}

pub fn far_call(
    vm: &mut VMState,
    opcode: &Opcode,
    far_call: &FarCallOpcode,
    storage: &dyn Storage,
) {
    /*
        TODO:
        - Read the code key from the contract deployer's storage with key `contract_address`.
        - Check the second byte from the code info; if it's a zero the call is a constructor code, if it's
            a one it's a regular runtime call.
        - Call decommit using the resulting code key returned from the deployer's storage instead.
    */
    let (src0, src1) = address_operands_read(vm, opcode);
    let contract_address = address_from_u256(&src1.value);
    let _err_routine = opcode.imm0;
    // TODO: PASS PROPERLY GAS FROM PARAMETERS
    let FarCallParams { ergs_passed, .. } =
        far_call_params_from_register(src0.value, vm.gas_left());
    match far_call {
        FarCallOpcode::Normal => {
            let program_code = storage
                .decommit(address_into_u256(contract_address))
                .unwrap();
            vm.push_far_call_frame(program_code, ergs_passed, &contract_address)
        }
        _ => todo!(),
    }
}

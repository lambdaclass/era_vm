use u256::{H160, U256};
use zkevm_opcode_defs::{
    ethereum_types::Address, system_params::DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW, FarCallOpcode,
};

use crate::{
    address_operands::address_operands_read,
    state::VMState,
    store::{Storage, StorageKey},
    utils::address_into_u256,
    value::FatPointer,
    Opcode,
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
        - Check constructor stuff.
    */

    let (src0, src1) = address_operands_read(vm, opcode);
    let contract_address = address_from_u256(&src1.value);
    let _err_routine = opcode.imm0;

    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let storage_key = StorageKey::new(
        deployer_system_contract_address,
        address_into_u256(contract_address),
    );

    let code_info = storage.storage_read(storage_key).unwrap();
    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    code_info_bytes[1] = 0;
    let code_key: U256 = U256::from_big_endian(&code_info_bytes);

    let FarCallParams { ergs_passed, .. } =
        far_call_params_from_register(src0.value, vm.gas_left());

    match far_call {
        FarCallOpcode::Normal => {
            let program_code = storage.decommit(code_key).unwrap();
            // TODO: manage calldata
            vm.push_far_call_frame(
                program_code,
                ergs_passed,
                contract_address,
                vm.current_frame().contract_address,
                vec![],
            )
        }
        _ => todo!(),
    }
}

use zkevm_opcode_defs::{
    system_params::{
        ECRECOVER_INNER_FUNCTION_PRECOMPILE_ADDRESS, KECCAK256_ROUND_FUNCTION_PRECOMPILE_ADDRESS,
        SECP256R1_VERIFY_PRECOMPILE_ADDRESS, SHA256_ROUND_FUNCTION_PRECOMPILE_ADDRESS,
    },
    PrecompileAuxData, PrecompileCallABI,
};

use crate::{
    address_operands::{address_operands_read, address_operands_store},
    eravm_error::EraVmError,
    execution::Execution,
    precompiles::{
        ecrecover::ecrecover_function, keccak256::keccak256_rounds_function,
        secp256r1_verify::secp256r1_verify_function, sha256::sha256_rounds_function,
    },
    state::VMState,
    statistics::VmStatistics,
    value::TaggedValue,
    Opcode,
};

pub fn precompile_call(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
    statistics: &mut VmStatistics,
) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;
    let aux_data = PrecompileAuxData::from_u256(src1.value);
    vm.decrease_gas(aux_data.extra_ergs_cost)?;

    state.add_pubdata(aux_data.extra_pubdata_cost as i32);

    let mut abi = PrecompileCallABI::from_u256(src0.value);
    if abi.memory_page_to_read == 0 {
        abi.memory_page_to_read = vm.current_context()?.heap_id;
    }
    if abi.memory_page_to_write == 0 {
        abi.memory_page_to_write = vm.current_context()?.heap_id;
    }
    let abi_key = abi.to_u256();

    let address_bytes = vm.current_context()?.contract_address.0;
    let address_low = u16::from_le_bytes([address_bytes[19], address_bytes[18]]);
    let heaps = &mut vm.heaps;

    match address_low {
        KECCAK256_ROUND_FUNCTION_PRECOMPILE_ADDRESS => {
            statistics.keccak256_cycles += keccak256_rounds_function(abi_key, heaps)?;
        }
        SHA256_ROUND_FUNCTION_PRECOMPILE_ADDRESS => {
            statistics.sha256_cycles += sha256_rounds_function(abi_key, heaps)?;
        }
        ECRECOVER_INNER_FUNCTION_PRECOMPILE_ADDRESS => {
            statistics.ecrecover_cycles += ecrecover_function(abi_key, heaps)?;
        }
        SECP256R1_VERIFY_PRECOMPILE_ADDRESS => {
            statistics.secp255r1_verify_cycles += secp256r1_verify_function(abi_key, heaps)?;
        }
        _ => {
            // A precompile call may be used just to burn gas
        }
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(1.into()))?;

    Ok(())
}

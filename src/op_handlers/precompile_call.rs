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
    value::TaggedValue,
    Opcode,
};

pub fn precompile_call(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;
    let aux_data = PrecompileAuxData::from_u256(src1.value);

    vm.decrease_gas(aux_data.extra_ergs_cost)?;

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
            keccak256_rounds_function(abi_key, heaps)?;
        }
        SHA256_ROUND_FUNCTION_PRECOMPILE_ADDRESS => {
            sha256_rounds_function(abi_key, heaps)?;
        }
        ECRECOVER_INNER_FUNCTION_PRECOMPILE_ADDRESS => {
            ecrecover_function(abi_key, heaps)?;
        }
        SECP256R1_VERIFY_PRECOMPILE_ADDRESS => {
            secp256r1_verify_function(abi_key, heaps)?;
        }
        _ => {
            // A precompile call may be used just to burn gas
        }
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(1.into()))?;

    Ok(())
}

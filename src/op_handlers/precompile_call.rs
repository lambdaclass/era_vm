use zk_evm_abstractions::{
    aux::Timestamp,
    precompiles::{
        ecrecover::ecrecover_function, keccak256::keccak256_rounds_function,
        secp256r1_verify::secp256r1_verify_function, sha256::sha256_rounds_function,
    },
    queries::LogQuery,
    vm::Memory,
};
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
    heaps::Heaps,
    state::VMState,
    value::TaggedValue,
    Opcode,
};

pub fn precompile_call(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
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

    let query = LogQuery {
        timestamp: Timestamp(0),
        key: abi.to_u256(),
        // only two first fields are read by the precompile
        tx_number_in_block: Default::default(),
        aux_byte: Default::default(),
        shard_id: Default::default(),
        address: Default::default(),
        read_value: Default::default(),
        written_value: Default::default(),
        rw_flag: Default::default(),
        rollback: Default::default(),
        is_service: Default::default(),
    };

    let address_bytes = vm.current_context()?.contract_address.0;
    let address_low = u16::from_le_bytes([address_bytes[19], address_bytes[18]]);
    let heaps = &mut vm.heaps;

    match address_low {
        KECCAK256_ROUND_FUNCTION_PRECOMPILE_ADDRESS => {
            keccak256_rounds_function::<_, false>(0, query, heaps);
        }
        SHA256_ROUND_FUNCTION_PRECOMPILE_ADDRESS => {
            sha256_rounds_function::<_, false>(0, query, heaps);
        }
        ECRECOVER_INNER_FUNCTION_PRECOMPILE_ADDRESS => {
            ecrecover_function::<_, false>(0, query, heaps);
        }
        SECP256R1_VERIFY_PRECOMPILE_ADDRESS => {
            secp256r1_verify_function::<_, false>(0, query, heaps);
        }
        _ => {
            // A precompile call may be used just to burn gas
        }
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(1.into()))?;

    Ok(())
}

impl Memory for Heaps {
    fn execute_partial_query(
        &mut self,
        _monotonic_cycle_counter: u32,
        mut query: zk_evm_abstractions::queries::MemoryQuery,
    ) -> zk_evm_abstractions::queries::MemoryQuery {
        let page = query.location.page.0;

        let start = query.location.index.0 * 32;
        if query.rw_flag {
            self.get_mut(page).unwrap().store(start, query.value);
        } else {
            self.get_mut(page).unwrap().expand_memory(start + 32);
            query.value = self.get(page).unwrap().read(start);
            query.value_is_pointer = false;
        }
        query
    }

    fn specialized_code_query(
        &mut self,
        _monotonic_cycle_counter: u32,
        _query: zk_evm_abstractions::queries::MemoryQuery,
    ) -> zk_evm_abstractions::queries::MemoryQuery {
        todo!()
    }

    fn read_code_query(
        &self,
        _monotonic_cycle_counter: u32,
        _query: zk_evm_abstractions::queries::MemoryQuery,
    ) -> zk_evm_abstractions::queries::MemoryQuery {
        todo!()
    }
}

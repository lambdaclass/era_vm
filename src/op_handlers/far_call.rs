use u256::{H160, U256};
use zkevm_opcode_defs::{
    ethereum_types::Address, system_params::DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW, FarCallOpcode,
};

use crate::{
    address_operands::address_operands_read,
    eravm_error::{EraVmError, HeapError},
    state::VMState,
    store::{Storage, StorageError, StorageKey},
    utils::{address_into_u256, is_kernel},
    value::{FatPointer, TaggedValue},
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
const FAR_CALL_GAS_SCALAR_MODIFIER_DIVIDEND: u32 = 63;
const FAR_CALL_GAS_SCALAR_MODIFIER_DIVISOR: u32 = 64;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum PointerSource {
    /// A new pointer for the heap
    NewForHeap = 0,
    /// An already existing, passed pointer.
    Forwarded = 1,
    /// A new pointer for the auxiliary heap
    NewForAuxHeap = 2,
}
impl PointerSource {
    pub fn from_abi(value: u8) -> Self {
        match value {
            1 => Self::Forwarded,
            2 => Self::NewForAuxHeap,
            _ => Self::NewForHeap,
        }
    }
}
pub fn get_forward_memory_pointer(
    source: U256,
    vm: &mut VMState,
    is_pointer: bool,
) -> Result<FatPointer, EraVmError> {
    let pointer_kind = PointerSource::from_abi((source.0[3] >> 32) as u8);
    let mut pointer = FatPointer::decode(source);
    match pointer_kind {
        PointerSource::Forwarded => {
            if !is_pointer || pointer.offset > pointer.len {
                return Err(EraVmError::NonValidForwardedMemory);
            }
            pointer.narrow();
        }
        PointerSource::NewForHeap | PointerSource::NewForAuxHeap => {
            // Check if the pointer is in bounds, otherwise, spend gas
            let Some(bound) = pointer.start.checked_add(pointer.len) else {
                vm.decrease_gas(u32::MAX)?;
                return Err(HeapError::StoreOutOfBounds.into());
            };

            if is_pointer || pointer.offset != 0 {
                return Err(HeapError::StoreOutOfBounds.into());
            }

            let ergs_cost = match pointer_kind {
                PointerSource::NewForHeap => {
                    pointer.page = vm.current_context()?.heap_id;
                    vm.heaps
                        .get_mut(vm.current_context()?.heap_id)
                        .ok_or(HeapError::StoreOutOfBounds)?
                        .expand_memory(bound)
                }
                PointerSource::NewForAuxHeap => {
                    pointer.page = vm.current_context()?.aux_heap_id;
                    vm.heaps
                        .get_mut(vm.current_context()?.aux_heap_id)
                        .ok_or(HeapError::StoreOutOfBounds)?
                        .expand_memory(pointer.start + pointer.len)
                }
                _ => unreachable!(),
            };

            let underflows = vm.decrease_gas(ergs_cost)?;
            if underflows {
                return Err(HeapError::StoreOutOfBounds.into());
            }
        }
    };
    Ok(pointer)
}

fn far_call_params_from_register(
    source: TaggedValue,
    vm: &mut VMState,
) -> Result<FarCallParams, EraVmError> {
    let is_pointer = source.is_pointer;
    let source = source.value;
    let mut args = [0u8; 32];
    let mut ergs_passed = source.0[3] as u32;
    let gas_left = vm.gas_left()?;

    if ergs_passed > gas_left {
        ergs_passed = ((gas_left as u64 * FAR_CALL_GAS_SCALAR_MODIFIER_DIVIDEND as u64)
            / FAR_CALL_GAS_SCALAR_MODIFIER_DIVISOR as u64) as u32;
    }
    source.to_little_endian(&mut args);
    let [.., shard_id, constructor_call_byte, system_call_byte] = args;

    let forward_memory = get_forward_memory_pointer(source, vm, is_pointer)?;

    Ok(FarCallParams {
        forward_memory,
        ergs_passed,
        constructor_call: constructor_call_byte != 0,
        to_system: system_call_byte != 0,
        shard_id,
    })
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
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;
    let contract_address = address_from_u256(&src1.value);

    let exception_handler = opcode.imm0 as u64;

    let abi = get_far_call_arguments(src0.value);

    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let storage_key = StorageKey::new(
        deployer_system_contract_address,
        address_into_u256(contract_address),
    );

    let code_info = storage
        .storage_read(storage_key)?
        .ok_or(StorageError::KeyNotPresent)?;
    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    // Note that EOAs are considered constructed because their code info is all zeroes.
    let is_constructed = match code_info_bytes[1] {
        0 => true,
        1 => false,
        _ => {
            return Err(EraVmError::IncorrectBytecodeFormat);
        }
    };
    let default_aa_code_hash: [u8; 32] = [1, 0, 5, 99, 55, 76, 39, 122, 44, 30, 52, 101, 154, 42, 30, 135, 55, 27, 182, 216, 82, 206, 20, 32, 34, 212, 151, 191, 181, 11, 158, 50];
    let try_default_aa = if is_kernel(&contract_address) {
        None
    } else {
        Some(default_aa_code_hash)
    };

    // The address aliasing contract implements Ethereum-like behavior of calls to EOAs
    // returning successfully (and address aliasing when called from the bootloader).
    // It makes sense that unconstructed code is treated as an EOA but for some reason
    // a constructor call to constructed code is also treated as EOA.
    code_info_bytes = match code_info_bytes[0] {
        1 => {
            if is_constructed == abi.is_constructor_call {
                try_default_aa.ok_or(StorageError::KeyNotPresent)?
            } else {
                code_info_bytes
            }
        }
        2 => {
            // This will change after 1.5 and evm_interpreter_code_hash should be used. Now there are the same.
            try_default_aa.ok_or(StorageError::KeyNotPresent)?
        }
        _ if code_info == U256::zero() => try_default_aa.ok_or(StorageError::KeyNotPresent)?,
        _ => return Err(EraVmError::IncorrectBytecodeFormat),
    };

    code_info_bytes[1] = 0;
    let code_key: U256 = U256::from_big_endian(&code_info_bytes);

    let FarCallParams {
        ergs_passed,
        forward_memory,
        ..
    } = far_call_params_from_register(src0, vm)?;

    match far_call {
        FarCallOpcode::Normal => {
            let program_code = storage
                .decommit(code_key)?
                .ok_or(StorageError::KeyNotPresent)?;
            let new_heap = vm.heaps.allocate();
            let new_aux_heap = vm.heaps.allocate();

            vm.push_far_call_frame(
                program_code,
                ergs_passed,
                contract_address,
                vm.current_context()?.contract_address,
                new_heap,
                new_aux_heap,
                forward_memory.page,
                exception_handler,
                vm.register_context_u128,
            )?;

            vm.register_context_u128 = 0_u128;

            if abi.is_system_call {
                // r3 to r12 are kept but they lose their pointer flags
                let zero = TaggedValue::zero();
                vm.set_register(13, zero);
                vm.set_register(14, zero);
                vm.set_register(15, zero);
                vm.clear_pointer_flags();
            } else {
                vm.clear_registers();
            }

            vm.clear_flags();

            let call_type = (u8::from(abi.is_system_call) << 1) | u8::from(abi.is_constructor_call);
            vm.set_register(2, TaggedValue::new_raw_integer(call_type.into()));

            // set calldata pointer
            vm.set_register(1, TaggedValue::new_pointer(forward_memory.encode()));
            Ok(())
        }
        FarCallOpcode::Mimic => {
            let program_code = storage
                .decommit(code_key)?
                .ok_or(StorageError::KeyNotPresent)?;
            let new_heap = vm.heaps.allocate();
            let new_aux_heap = vm.heaps.allocate();

            let mut caller_bytes = [0; 32];
            let caller = vm.get_register(15).value;
            caller.to_big_endian(&mut caller_bytes);

            let mut caller_bytes_20: [u8; 20] = [0; 20];
            for (i, byte) in caller_bytes[12..].iter().enumerate() {
                caller_bytes_20[i] = *byte;
            }

            vm.push_far_call_frame(
                program_code,
                ergs_passed,
                contract_address,
                H160::from(caller_bytes_20),
                new_heap,
                new_aux_heap,
                forward_memory.page,
                exception_handler,
                vm.register_context_u128,
            )?;

            vm.register_context_u128 = 0_u128;

            if abi.is_system_call {
                // r3 to r12 are kept but they lose their pointer flags
                let zero = TaggedValue::zero();
                vm.set_register(13, zero);
                vm.set_register(14, zero);
                vm.set_register(15, zero);
                vm.clear_pointer_flags();
            } else {
                vm.clear_registers();
            }

            vm.clear_flags();

            let call_type = (u8::from(abi.is_system_call) << 1) | u8::from(abi.is_constructor_call);
            vm.set_register(2, TaggedValue::new_raw_integer(call_type.into()));

            // set calldata pointer
            vm.set_register(1, TaggedValue::new_pointer(forward_memory.encode()));
            vm.current_context_mut()?.caller = address_from_u256(&vm.get_register(15).value);
            Ok(())
        }
        _ => todo!(),
    }
}

pub(crate) struct FarCallABI {
    pub _gas_to_pass: u32,
    pub _shard_id: u8,
    pub is_constructor_call: bool,
    pub is_system_call: bool,
}

pub(crate) fn get_far_call_arguments(abi: U256) -> FarCallABI {
    let _gas_to_pass = abi.0[3] as u32;
    let settings = (abi.0[3] >> 32) as u32;
    let [_, _shard_id, constructor_call_byte, system_call_byte] = settings.to_le_bytes();

    FarCallABI {
        _gas_to_pass,
        _shard_id,
        is_constructor_call: constructor_call_byte != 0,
        is_system_call: system_call_byte != 0,
    }
}

pub fn perform_return(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let register = vm.get_register(opcode.src0_index);
    let result = get_forward_memory_pointer(register.value, vm, register.is_pointer)?;
    vm.set_register(
        opcode.src0_index,
        TaggedValue::new_pointer(FatPointer::encode(&result)),
    );
    Ok(())
}

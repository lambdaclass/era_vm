
use u256::{H160, U256};
use zkevm_opcode_defs::{
    ethereum_types::Address,
    system_params::{
        DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW, EVM_SIMULATOR_STIPEND,
        MSG_VALUE_SIMULATOR_ADDITIVE_COST,
    },
    FarCallOpcode, ADDRESS_MSG_VALUE,
};

use crate::{
    address_operands::address_operands_read,
    eravm_error::{EraVmError, HeapError},
    execution::Execution,
    rollbacks::Rollbackable,
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
    vm: &mut Execution,
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

            if !is_pointer && pointer.offset == 0 {
                pointer.page = if let PointerSource::NewForHeap = pointer_kind {
                    vm.current_context()?.heap_id
                } else {
                    vm.current_context()?.aux_heap_id
                };

                let ergs_cost = vm
                    .heaps
                    .get_mut(pointer.page)
                    .ok_or(HeapError::StoreOutOfBounds)?
                    .expand_memory(bound);

                vm.decrease_gas(ergs_cost)?;
            }
        }
    };
    Ok(pointer)
}

fn far_call_params_from_register(
    source: TaggedValue,
    vm: &mut Execution,
) -> Result<FarCallParams, EraVmError> {
    let is_pointer = source.is_pointer;
    let source = source.value;
    let mut args = [0u8; 32];
    let mut ergs_passed = source.0[3] as u32;
    let gas_left = vm.gas_left()?;

    let maximum_gas =
        gas_left / FAR_CALL_GAS_SCALAR_MODIFIER_DIVISOR * FAR_CALL_GAS_SCALAR_MODIFIER_DIVIDEND;

    ergs_passed = ergs_passed.min(maximum_gas);

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

fn decommit_code_hash(
    state: &mut VMState,
    address: Address,
    default_aa_code_hash: [u8; 32],
    evm_interpreter_code_hash: [u8; 32],
    is_constructor_call: bool,
    storage: &mut dyn Storage,
) -> Result<(U256, bool, u32), EraVmError> {
    let mut is_evm = false;
    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let storage_key = StorageKey::new(deployer_system_contract_address, address_into_u256(address));

    // reading when decommiting doesn't refund
    let code_info = state.storage_read_with_no_refund(storage_key, storage);
    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    const IS_CONSTRUCTED_FLAG_ON: u8 = 0;
    const IS_CONSTRUCTED_FLAG_OFF: u8 = 1;

    // Note that EOAs are considered constructed because their code info is all zeroes.
    let is_constructed = match code_info_bytes[1] {
        IS_CONSTRUCTED_FLAG_ON => true,
        IS_CONSTRUCTED_FLAG_OFF => false,
        _ => {
            return Err(EraVmError::IncorrectBytecodeFormat);
        }
    };

    // We won't mask the address if it belongs to kernel
    let is_not_kernel = !is_kernel(&address);
    let try_default_aa = is_not_kernel.then_some(default_aa_code_hash);

    const CONTRACT_VERSION_FLAG: u8 = 1;
    const BLOB_VERSION_FLAG: u8 = 2;

    // The address aliasing contract implements Ethereum-like behavior of calls to EOAs
    // returning successfully (and address aliasing when called from the bootloader).
    // It makes sense that unconstructed code is treated as an EOA but for some reason
    // a constructor call to constructed code is also treated as EOA.
    code_info_bytes = match code_info_bytes[0] {
        // There is an ERA VM contract stored in this address
        CONTRACT_VERSION_FLAG => {
            // If we pretend to call the constructor, and it hasn't been already constructed, then
            // we proceed. In other case, we return default.
            // If we pretend to call a normal function, and it has been already constructed, then
            // we proceed. In other case, we return default.
            if is_constructed == is_constructor_call {
                try_default_aa.ok_or(StorageError::KeyNotPresent)?
            } else {
                code_info_bytes
            }
        }
        // There is an EVM contract (blob) stored in this address (we need the interpreter)
        BLOB_VERSION_FLAG => {
            if is_constructed == is_constructor_call {
                try_default_aa.ok_or(StorageError::KeyNotPresent)?
            } else {
                is_evm = true;
                evm_interpreter_code_hash
            }
        }
        // EOA: There is no code, so we return the default
        _ if code_info == U256::zero() => try_default_aa.ok_or(StorageError::KeyNotPresent)?,
        // Invalid
        _ => return Err(EraVmError::IncorrectBytecodeFormat),
    };

    code_info_bytes[1] = 0;

    let code_key = U256::from_big_endian(&code_info_bytes);

    let cost = if state.decommitted_hashes().contains(&code_key) {
        0
    } else {
        let code_length_in_words = u16::from_be_bytes([code_info_bytes[2], code_info_bytes[3]]);
        code_length_in_words as u32 * zkevm_opcode_defs::ERGS_PER_CODE_WORD_DECOMMITTMENT
    };

    Ok((U256::from_big_endian(&code_info_bytes), is_evm, cost))
}

pub fn far_call(
    vm: &mut Execution,
    opcode: &Opcode,
    far_call: &FarCallOpcode,
    state: &mut VMState,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;
    let contract_address = address_from_u256(&src1.value);

    let exception_handler = opcode.imm0 as u64;
    let snapshot = state.snapshot();

    let mut abi = get_far_call_arguments(src0.value);
    abi.is_constructor_call = abi.is_constructor_call && vm.current_context()?.is_kernel();
    abi.is_system_call = abi.is_system_call && is_kernel(&contract_address);

    let (code_key, is_evm, decommit_cost) = decommit_code_hash(
        state,
        contract_address,
        vm.default_aa_code_hash,
        vm.evm_interpreter_code_hash,
        abi.is_constructor_call,
        storage,
    )?;

    // Unlike all other gas costs, this one is not paid if low on gas.
    if decommit_cost <= vm.gas_left()? {
        vm.decrease_gas(decommit_cost)?;
    } else {
        return Err(EraVmError::DecommitFailed);
    }

    let FarCallParams {
        ergs_passed,
        forward_memory,
        ..
    } = far_call_params_from_register(src0, vm)?;

    let mandated_gas = if abi.is_system_call && src1.value == ADDRESS_MSG_VALUE.into() {
        MSG_VALUE_SIMULATOR_ADDITIVE_COST
    } else {
        0
    };

    // mandated gas can surprass the 63/64 limit
    let ergs_passed = ergs_passed + mandated_gas;

    vm.decrease_gas(ergs_passed)?;

    let stipend = if is_evm { EVM_SIMULATOR_STIPEND } else { 0 };

    let ergs_passed = (ergs_passed)
        .checked_add(stipend)
        .expect("stipend must not cause overflow");

    let program_code = state
        .decommit(code_key, storage)
        .0
        .ok_or(StorageError::KeyNotPresent)?;
    let new_heap = vm.heaps.allocate();
    let new_aux_heap = vm.heaps.allocate();
    let is_new_frame_static = opcode.flag0_set || vm.current_context()?.is_static;

    match far_call {
        FarCallOpcode::Normal => {
            vm.push_far_call_frame(
                program_code,
                ergs_passed,
                contract_address,
                contract_address,
                vm.current_context()?.contract_address,
                new_heap,
                new_aux_heap,
                forward_memory.page,
                exception_handler,
                vm.register_context_u128,
                snapshot,
                is_new_frame_static && !is_evm,
                stipend,
            )?;
        }
        FarCallOpcode::Mimic => {
            let caller = address_from_u256(&vm.get_register(15).value);

            vm.push_far_call_frame(
                program_code,
                ergs_passed,
                contract_address,
                contract_address,
                caller,
                new_heap,
                new_aux_heap,
                forward_memory.page,
                exception_handler,
                vm.register_context_u128,
                snapshot,
                is_new_frame_static && !is_evm,
                stipend,
            )?;
        }
        FarCallOpcode::Delegate => {
            let this_context = vm.current_context()?;
            let this_contract_address = this_context.contract_address;

            vm.push_far_call_frame(
                program_code,
                ergs_passed,
                contract_address,
                this_contract_address,
                this_context.caller,
                new_heap,
                new_aux_heap,
                forward_memory.page,
                exception_handler,
                this_context.context_u128,
                snapshot,
                is_new_frame_static && !is_evm,
                stipend,
            )?;
        }
    };

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

    let is_static_call_to_evm_interpreter = is_new_frame_static && is_evm;
    let call_type = (u8::from(is_static_call_to_evm_interpreter) << 2)
        | (u8::from(abi.is_system_call) << 1)
        | u8::from(abi.is_constructor_call);
    vm.set_register(2, TaggedValue::new_raw_integer(call_type.into()));

    // set calldata pointer
    vm.set_register(1, TaggedValue::new_pointer(forward_memory.encode()));
    Ok(())
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

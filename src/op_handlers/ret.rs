use u256::U256;
use zkevm_opcode_defs::RetOpcode;

use crate::{
    eravm_error::EraVmError,
    state::VMState,
    store::{InMemory, Storage},
    value::{FatPointer, TaggedValue},
    Opcode,
};

use super::far_call::get_forward_memory_pointer;

fn is_failure(return_type: RetOpcode) -> bool {
    return_type != RetOpcode::Ok
}

fn get_result(
    vm: &mut VMState,
    reg_index: u8,
    return_type: RetOpcode,
) -> Result<TaggedValue, EraVmError> {
    if return_type == RetOpcode::Panic {
        return Ok(TaggedValue::new_pointer(U256::zero()));
    }
    let register = vm.get_register(reg_index);
    let result = get_forward_memory_pointer(register.value, vm, register.is_pointer)?;
    if !vm.current_context()?.is_kernel() && result.page == vm.current_context()?.calldata_heap_id {
        return Err(EraVmError::InvalidCalldataAccess);
    }
    Ok(TaggedValue::new_pointer(FatPointer::encode(&result)))
}

fn save_transient_store(vm: &mut VMState, prev_storage: InMemory) -> Result<(), EraVmError> {
    let keys = prev_storage.get_all_keys();
    for key in keys {
        let value = prev_storage.storage_read(key)?;
        if let Some(value) = value {
            vm.current_frame_mut()?
                .transient_storage
                .storage_write(key, value)?;
        }
    }

    Ok(())
}

pub fn ret(
    vm: &mut VMState,
    opcode: &Opcode,
    storage: &mut dyn Storage,
    return_type: RetOpcode,
) -> Result<bool, EraVmError> {
    let is_failure = is_failure(return_type);

    vm.flag_eq = false;
    vm.flag_lt_of = return_type == RetOpcode::Panic;
    vm.flag_gt = false;

    if is_failure {
        storage.rollback(&vm.current_frame()?.storage_before);
    }

    if vm.in_near_call()? {
        let previous_frame = vm.pop_frame()?;
        if opcode.flag0_set {
            let to_label = opcode.imm0;
            vm.current_frame_mut()?.pc = to_label as u64;
        } else if is_failure {
            vm.current_frame_mut()?.pc = previous_frame.exception_handler;
        } else {
            vm.current_frame_mut()?.pc += 1;
        }
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        if return_type == RetOpcode::Ok {
            save_transient_store(vm, *previous_frame.transient_storage)?;
        }
        Ok(false)
    } else if vm.in_far_call() {
        let result = get_result(vm, opcode.src0_index, return_type)?;
        vm.register_context_u128 = 0_u128;
        vm.clear_registers();
        vm.set_register(1, result);
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        if is_failure {
            vm.current_frame_mut()?.pc = previous_frame.exception_handler;
        } else {
            vm.current_frame_mut()?.pc += 1;
        }

        if return_type == RetOpcode::Ok {
            save_transient_store(vm, *previous_frame.transient_storage)?;
        }

        Ok(false)
    } else {
        if return_type == RetOpcode::Panic {
            return Ok(true);
        }
        let result = get_result(vm, opcode.src0_index, return_type)?;
        vm.set_register(1, result);
        Ok(true)
    }
}

pub fn inexplicit_panic(vm: &mut VMState, storage: &mut dyn Storage) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;

    storage.rollback(&vm.current_frame()?.storage_before);

    if vm.in_near_call()? {
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.pc = previous_frame.exception_handler;
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;

        Ok(false)
    } else if vm.in_far_call() {
        let result = TaggedValue::new_pointer(U256::zero());
        vm.register_context_u128 = 0_u128;
        vm.clear_registers();
        vm.set_register(1, result);
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        vm.current_frame_mut()?.pc = previous_frame.exception_handler;
        Ok(false)
    } else {
        Ok(true)
    }
}

// When executing a far_call, if the opcode fails, we need to run the exception handler provided in the args
// We don't need to: run ret.panic, pop a frame and run its exception handler
pub fn panic_from_far_call(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let far_call_exception_handler = opcode.imm0 as u64;
    let result = TaggedValue::new_pointer(U256::zero());
    vm.register_context_u128 = 0_u128;
    vm.clear_registers();
    vm.set_register(1, result);
    vm.current_frame_mut()?.pc = far_call_exception_handler;

    Ok(())
}

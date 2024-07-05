mod address_operands;
pub mod call_frame;
mod eravm_error;
mod op_handlers;
mod opcode;
pub mod output;
mod ptr_operator;
pub mod state;
pub mod store;
pub mod tracers;
pub mod value;

use eravm_error::EraVmError;
use op_handlers::add::add;
use op_handlers::and::and;
use op_handlers::aux_heap_read::aux_heap_read;
use op_handlers::aux_heap_write::aux_heap_write;
use op_handlers::div::div;
use op_handlers::far_call::far_call;
use op_handlers::fat_pointer_read::fat_pointer_read;
use op_handlers::heap_read::heap_read;
use op_handlers::heap_write::heap_write;
use op_handlers::jump::jump;
use op_handlers::log::{
    storage_read, storage_write, transient_storage_read, transient_storage_write,
};

use op_handlers::mul::mul;
use op_handlers::near_call::near_call;
use op_handlers::ok::ok;
use op_handlers::or::or;
use op_handlers::panic::panic;
use op_handlers::ptr_add::ptr_add;
use op_handlers::ptr_pack::ptr_pack;
use op_handlers::ptr_shrink::ptr_shrink;
use op_handlers::ptr_sub::ptr_sub;
use op_handlers::revert::{handle_error, revert, revert_out_of_gas};
use op_handlers::shift::rol;
use op_handlers::shift::ror;
use op_handlers::shift::shl;
use op_handlers::shift::shr;
use op_handlers::sub::sub;
use op_handlers::xor::xor;
pub use opcode::Opcode;
use output::Output;
use state::VMState;
use tracers::tracer::Tracer;
use u256::U256;
use zkevm_opcode_defs::definitions::synthesize_opcode_decoding_tables;
use zkevm_opcode_defs::ISAVersion;
use zkevm_opcode_defs::LogOpcode;
use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::PtrOpcode;
use zkevm_opcode_defs::ShiftOpcode;
use zkevm_opcode_defs::UMAOpcode;
use zkevm_opcode_defs::{BinopOpcode, RetOpcode};

/// Run a vm program from the given path using a custom state.
/// Returns the value stored at storage with key 0 and the final vm state.
pub fn program_from_file(bin_path: &str) -> Result<Vec<U256>, EraVmError> {
    let program = std::fs::read(bin_path)?;
    let encoded =
        String::from_utf8(program.to_vec()).map_err(|_| EraVmError::IncorrectBytecodeFormat)?;
    let bin = hex::decode(&encoded[2..]).map_err(|_| EraVmError::IncorrectBytecodeFormat)?;

    let mut program_code = vec![];
    for raw_opcode_slice in bin.chunks(32) {
        let mut raw_opcode_bytes: [u8; 32] = [0; 32];
        raw_opcode_bytes.copy_from_slice(&raw_opcode_slice[..32]);

        let raw_opcode_u256 = U256::from_big_endian(&raw_opcode_bytes);
        program_code.push(raw_opcode_u256);
    }
    Ok(program_code)
}

/// Run a vm program with a clean VM state.
pub fn run_program(bin_path: &str, vm: VMState, tracers: &mut [Box<&mut dyn Tracer>]) -> Output {
    match run_program_with_error(bin_path, vm, tracers) {
        Ok((storage, vm)) => Output {
            storage_zero: storage,
            vm_state: vm,
            reverted: false,
            reason: None,
        },
        Err(e) => Output {
            storage_zero: U256::zero(),
            vm_state: VMState::default(),
            reverted: true,
            reason: Some(e),
        },
    }
}

pub fn run_program_with_error(
    bin_path: &str,
    mut vm: VMState,
    tracers: &mut [Box<&mut dyn Tracer>],
) -> Result<(U256, VMState), EraVmError> {
    let program_code = program_from_file(bin_path)?;
    vm.load_program(program_code);
    run(vm, tracers)
}

pub fn run(
    mut vm: VMState,
    tracers: &mut [Box<&mut dyn Tracer>],
) -> Result<(U256, VMState), EraVmError> {
    let opcode_table = synthesize_opcode_decoding_tables(11, ISAVersion(2));
    loop {
        let opcode = vm.get_opcode(&opcode_table)?;
        for tracer in tracers.iter_mut() {
            tracer.before_execution(&opcode, &vm);
        }
        let gas_underflows = vm.decrease_gas(&opcode)?;

        if vm.predicate_holds(&opcode.predicate) {
            let result = match opcode.variant {
                // TODO: Properly handle what happens
                // when the VM runs out of ergs/gas.
                _ if vm.running_contexts.len() == 1
                    && vm.current_context()?.near_call_frames.is_empty()
                    && gas_underflows =>
                {
                    break;
                }
                _ if gas_underflows => revert_out_of_gas(&mut vm),
                Variant::Invalid(_) => todo!(),
                Variant::Nop(_) => todo!(),
                Variant::Add(_) => add(&mut vm, &opcode),
                Variant::Sub(_) => sub(&mut vm, &opcode),
                Variant::Jump(_) => jump(&mut vm, &opcode),
                Variant::Mul(_) => mul(&mut vm, &opcode),
                Variant::Div(_) => div(&mut vm, &opcode),
                Variant::Context(_) => todo!(),
                Variant::Shift(shift_variant) => match shift_variant {
                    ShiftOpcode::Shl => shl(&mut vm, &opcode),
                    ShiftOpcode::Shr => shr(&mut vm, &opcode),
                    ShiftOpcode::Rol => rol(&mut vm, &opcode),
                    ShiftOpcode::Ror => ror(&mut vm, &opcode),
                },
                Variant::Binop(binop) => match binop {
                    BinopOpcode::Xor => xor(&mut vm, &opcode),
                    BinopOpcode::And => and(&mut vm, &opcode),
                    BinopOpcode::Or => or(&mut vm, &opcode),
                },
                Variant::Ptr(ptr_variant) => match ptr_variant {
                    PtrOpcode::Add => ptr_add(&mut vm, &opcode),
                    PtrOpcode::Sub => ptr_sub(&mut vm, &opcode),
                    PtrOpcode::Pack => ptr_pack(&mut vm, &opcode),
                    PtrOpcode::Shrink => ptr_shrink(&mut vm, &opcode),
                },
                Variant::NearCall(_) => near_call(&mut vm, &opcode),
                Variant::Log(log_variant) => match log_variant {
                    LogOpcode::StorageRead => storage_read(&mut vm, &opcode),
                    LogOpcode::StorageWrite => storage_write(&mut vm, &opcode),
                    LogOpcode::ToL1Message => todo!(),
                    LogOpcode::Event => todo!(),
                    LogOpcode::PrecompileCall => todo!(),
                    LogOpcode::Decommit => todo!(),
                    LogOpcode::TransientStorageRead => transient_storage_read(&mut vm, &opcode),
                    LogOpcode::TransientStorageWrite => transient_storage_write(&mut vm, &opcode),
                },
                Variant::FarCall(far_call_variant) => far_call(&mut vm, &far_call_variant),
                // TODO: This is not how return works. Fix when we have calls between contracts
                // hooked up.
                // This is only to keep the context for tests
                Variant::Ret(ret_variant) => match ret_variant {
                    RetOpcode::Ok => {
                        let should_break = ok(&mut vm, &opcode)?;
                        if should_break {
                            break;
                        }
                        Ok(())
                    }
                    RetOpcode::Revert => {
                        let should_break = revert(&mut vm, &opcode)?;
                        if should_break {
                            panic!("Contract Reverted");
                        };
                        Ok(())
                    }
                    RetOpcode::Panic => {
                        let should_break = panic(&mut vm, &opcode)?;
                        if should_break {
                            panic!("Contract Panicked");
                        };
                        Ok(())
                    }
                },
                Variant::UMA(uma_variant) => match uma_variant {
                    UMAOpcode::HeapRead => heap_read(&mut vm, &opcode),
                    UMAOpcode::HeapWrite => heap_write(&mut vm, &opcode),
                    UMAOpcode::AuxHeapRead => aux_heap_read(&mut vm, &opcode),
                    UMAOpcode::AuxHeapWrite => aux_heap_write(&mut vm, &opcode),
                    UMAOpcode::FatPointerRead => fat_pointer_read(&mut vm, &opcode),
                    UMAOpcode::StaticMemoryRead => todo!(),
                    UMAOpcode::StaticMemoryWrite => todo!(),
                },
            };
            if let Err(e) = result {
                handle_error(&mut vm, e)?;
            }
        };
        vm.current_frame_mut()?.pc += 1;
    }
    let final_storage_value = vm.current_frame()?.storage.borrow().read(&U256::zero())?;
    Ok((final_storage_value, vm))
}

mod address_operands;
pub mod call_frame;
mod op_handlers;
mod opcode;
mod ptr_operator;
pub mod state;
pub mod store;
pub mod tracers;
pub mod utils;
pub mod value;

use op_handlers::add::_add;
use op_handlers::and::_and;
use op_handlers::aux_heap_read::_aux_heap_read;
use op_handlers::aux_heap_write::_aux_heap_write;
use op_handlers::div::_div;
use op_handlers::far_call::far_call;
use op_handlers::fat_pointer_read::_fat_pointer_read;
use op_handlers::heap_read::_heap_read;
use op_handlers::heap_write::_heap_write;
use op_handlers::jump::_jump;
use op_handlers::log::{
    _storage_read, _storage_write, _transient_storage_read, _transient_storage_write,
};
use op_handlers::mul::_mul;
use op_handlers::near_call::_near_call;
use op_handlers::ok::_ok;
use op_handlers::or::_or;
use op_handlers::panic::_panic;
use op_handlers::ptr_add::_ptr_add;
use op_handlers::ptr_pack::_ptr_pack;
use op_handlers::ptr_shrink::_ptr_shrink;
use op_handlers::ptr_sub::_ptr_sub;
use op_handlers::revert::{_revert, _revert_out_of_gas};
use op_handlers::shift::_rol;
use op_handlers::shift::_ror;
use op_handlers::shift::_shl;
use op_handlers::shift::_shr;
use op_handlers::sub::_sub;
use op_handlers::xor::_xor;
pub use opcode::Opcode;
use state::VMState;
use store::Storage;
use tracers::tracer::Tracer;
use u256::U256;
use zkevm_opcode_defs::LogOpcode;
use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::PtrOpcode;
use zkevm_opcode_defs::ShiftOpcode;
use zkevm_opcode_defs::UMAOpcode;
use zkevm_opcode_defs::{synthesize_opcode_decoding_tables, ISAVersion};
use zkevm_opcode_defs::{BinopOpcode, RetOpcode};

/// Run a vm program from the given path using a custom state.
/// Returns the value stored at storage with key 0 and the final vm state.
pub fn program_from_file(bin_path: &str) -> Vec<U256> {
    let program = std::fs::read(bin_path).unwrap();
    let encoded = String::from_utf8(program.to_vec()).unwrap();
    let bin = hex::decode(&encoded[2..]).unwrap();

    let mut program_code = vec![];
    for raw_opcode_slice in bin.chunks(32) {
        let mut raw_opcode_bytes: [u8; 32] = [0; 32];
        raw_opcode_bytes.copy_from_slice(&raw_opcode_slice[..32]);

        let raw_opcode_u256 = U256::from_big_endian(&raw_opcode_bytes);
        program_code.push(raw_opcode_u256);
    }
    program_code
}

/// Run a vm program with a clean VM state.
pub fn run_program(
    bin_path: &str,
    mut vm: VMState,
    storage: &mut dyn Storage,
    tracers: &mut [Box<&mut dyn Tracer>],
) -> (U256, VMState) {
    let program_code = program_from_file(bin_path);
    vm.load_program(program_code);
    run(vm, storage, tracers)
}

pub fn run(
    mut vm: VMState,
    storage: &mut dyn Storage,
    tracers: &mut [Box<&mut dyn Tracer>],
) -> (U256, VMState) {
    let opcode_table = synthesize_opcode_decoding_tables(11, ISAVersion(2));
    let contract_address = vm.current_frame().contract_address;
    loop {
        let opcode = vm.get_opcode(&opcode_table);
        for tracer in tracers.iter_mut() {
            tracer.before_execution(&opcode, &mut vm, storage);
        }
        let gas_underflows = vm.decrease_gas(&opcode);
        if vm.predicate_holds(&opcode.predicate) {
            match opcode.variant {
                // TODO: Properly handle what happens
                // when the VM runs out of ergs/gas.
                _ if vm.running_contexts.len() == 1
                    && vm.current_context().near_call_frames.is_empty()
                    && gas_underflows =>
                {
                    break
                }
                _ if gas_underflows => {
                    _revert_out_of_gas(&mut vm);
                }
                Variant::Invalid(_) => todo!(),
                Variant::Nop(_) => todo!(),
                Variant::Add(_) => {
                    _add(&mut vm, &opcode);
                }
                Variant::Sub(_) => _sub(&mut vm, &opcode),
                Variant::Jump(_) => _jump(&mut vm, &opcode),
                Variant::Mul(_) => _mul(&mut vm, &opcode),
                Variant::Div(_) => _div(&mut vm, &opcode),
                Variant::Context(_) => todo!(),
                Variant::Shift(_) => match opcode.variant {
                    Variant::Shift(ShiftOpcode::Shl) => _shl(&mut vm, &opcode),
                    Variant::Shift(ShiftOpcode::Shr) => _shr(&mut vm, &opcode),
                    Variant::Shift(ShiftOpcode::Rol) => _rol(&mut vm, &opcode),
                    Variant::Shift(ShiftOpcode::Ror) => _ror(&mut vm, &opcode),
                    _ => unreachable!(),
                },
                Variant::Binop(binop) => match binop {
                    BinopOpcode::Xor => _xor(&mut vm, &opcode),
                    BinopOpcode::And => _and(&mut vm, &opcode),
                    BinopOpcode::Or => _or(&mut vm, &opcode),
                },
                Variant::Ptr(ptr_variant) => match ptr_variant {
                    PtrOpcode::Add => _ptr_add(&mut vm, &opcode),
                    PtrOpcode::Sub => _ptr_sub(&mut vm, &opcode),
                    PtrOpcode::Pack => _ptr_pack(&mut vm, &opcode),
                    PtrOpcode::Shrink => _ptr_shrink(&mut vm, &opcode),
                },
                Variant::NearCall(_) => _near_call(&mut vm, &opcode),
                Variant::Log(log_variant) => match log_variant {
                    LogOpcode::StorageRead => _storage_read(&mut vm, &opcode, storage),
                    LogOpcode::StorageWrite => _storage_write(&mut vm, &opcode, storage),
                    LogOpcode::ToL1Message => todo!(),
                    LogOpcode::Event => todo!(),
                    LogOpcode::PrecompileCall => todo!(),
                    LogOpcode::Decommit => todo!(),
                    LogOpcode::TransientStorageRead => _transient_storage_read(&mut vm, &opcode),
                    LogOpcode::TransientStorageWrite => _transient_storage_write(&mut vm, &opcode),
                },
                Variant::FarCall(far_call_variant) => {
                    far_call(&mut vm, &opcode, &far_call_variant, storage)
                }
                // TODO: This is not how return works. Fix when we have calls between contracts
                // hooked up.
                // This is only to keep the context for tests
                Variant::Ret(ret_variant) => match ret_variant {
                    RetOpcode::Ok => {
                        let should_break = _ok(&mut vm, &opcode);
                        if should_break {
                            break;
                        }
                    }
                    RetOpcode::Revert => {
                        let should_break = _revert(&mut vm, &opcode);
                        if should_break {
                            panic!("Contract Reverted");
                        }
                    }
                    RetOpcode::Panic => {
                        let should_break = _panic(&mut vm, &opcode);
                        if should_break {
                            panic!("Contract Panicked");
                        }
                    }
                },

                Variant::UMA(uma_variant) => match uma_variant {
                    UMAOpcode::HeapRead => _heap_read(&mut vm, &opcode),
                    UMAOpcode::HeapWrite => _heap_write(&mut vm, &opcode),
                    UMAOpcode::AuxHeapRead => _aux_heap_read(&mut vm, &opcode),
                    UMAOpcode::AuxHeapWrite => _aux_heap_write(&mut vm, &opcode),
                    UMAOpcode::FatPointerRead => _fat_pointer_read(&mut vm, &opcode),
                    UMAOpcode::StaticMemoryRead => todo!(),
                    UMAOpcode::StaticMemoryWrite => todo!(),
                },
            }
        }
        vm.current_frame_mut().pc = opcode_pc_set(&opcode, vm.current_frame().pc);
    }

    let final_storage_value = match storage.storage_read((contract_address, U256::zero())) {
        Some(value) => value,
        None => U256::zero(),
    };
    (final_storage_value, vm)
}

// Set the next PC according to th enext opcode
fn opcode_pc_set(opcode: &Opcode, current_pc: u64) -> u64 {
    match opcode.variant {
        Variant::FarCall(_) => 0,
        _ => current_pc + 1,
    }
}

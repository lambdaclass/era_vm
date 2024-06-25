mod address_operands;
mod op_handlers;
mod opcode;
mod ptr_operator;
pub mod state;
mod store;
pub mod value;

use std::path::PathBuf;

use op_handlers::add::_add;
use op_handlers::div::_div;
use op_handlers::log::{
    _storage_read, _storage_write, _transient_storage_read, _transient_storage_write,
};
use op_handlers::mul::_mul;
use op_handlers::ptr_add::_ptr_add;
use op_handlers::ptr_pack::_ptr_pack;
use op_handlers::ptr_shrink::_ptr_shrink;
use op_handlers::ptr_sub::_ptr_sub;
use op_handlers::sub::_sub;
pub use opcode::Opcode;
use state::{VMState, VMStateBuilder};
use u256::U256;
use zkevm_opcode_defs::definitions::synthesize_opcode_decoding_tables;
use zkevm_opcode_defs::ISAVersion;
use zkevm_opcode_defs::LogOpcode;
use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::PtrOpcode;

/// Run a vm program with a clean VM state and with in memory storage.
pub fn run_program_in_memory(bin_path: &str) -> (U256, VMState) {
    let vm = VMStateBuilder::default().build();
    run_program_with_custom_state(bin_path, vm)
}

/// Run a vm program saving the state to a storage file at the given path.
pub fn run_program_with_storage(bin_path: &str, storage_path: String) -> (U256, VMState) {
    let vm = VMStateBuilder::default()
        .with_storage(PathBuf::from(storage_path))
        .build();
    run_program_with_custom_state(bin_path, vm)
}

/// Run a vm program from the given path using a custom state.
/// Returns the value stored at storage with key 0 and the final vm state.
pub fn run_program_with_custom_state(bin_path: &str, mut vm: VMState) -> (U256, VMState) {
    let opcode_table = synthesize_opcode_decoding_tables(11, ISAVersion(2));

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

    vm.load_program(program_code);

    loop {
        let opcode = vm.get_opcode(&opcode_table);

        if vm.predicate_holds(&opcode.predicate) {
            match opcode.variant {
                // TODO: Properly handle what happens
                // when the VM runs out of ergs/gas.
                _ if vm.gas_left() == 0 => break,
                Variant::Invalid(_) => todo!(),
                Variant::Nop(_) => todo!(),
                Variant::Add(_) => {
                    _add(&mut vm, &opcode);
                }
                Variant::Sub(_) => _sub(&mut vm, &opcode),
                Variant::Mul(_) => _mul(&mut vm, &opcode),
                Variant::Div(_) => _div(&mut vm, &opcode),
                Variant::Jump(_) => todo!(),
                Variant::Context(_) => todo!(),
                Variant::Shift(_) => todo!(),
                Variant::Binop(_) => todo!(),
                Variant::Ptr(ptr_variant) => match ptr_variant {
                    PtrOpcode::Add => _ptr_add(&mut vm, &opcode),
                    PtrOpcode::Sub => _ptr_sub(&mut vm, &opcode),
                    PtrOpcode::Pack => _ptr_pack(&mut vm, &opcode),
                    PtrOpcode::Shrink => _ptr_shrink(&mut vm, &opcode),
                },
                Variant::NearCall(_) => todo!(),
                Variant::Log(log_variant) => match log_variant {
                    LogOpcode::StorageRead => _storage_read(&mut vm, &opcode),
                    LogOpcode::StorageWrite => _storage_write(&mut vm, &opcode),
                    LogOpcode::ToL1Message => todo!(),
                    LogOpcode::Event => todo!(),
                    LogOpcode::PrecompileCall => todo!(),
                    LogOpcode::Decommit => todo!(),
                    LogOpcode::TransientStorageRead => _transient_storage_read(&mut vm, &opcode),
                    LogOpcode::TransientStorageWrite => _transient_storage_write(&mut vm, &opcode),
                },
                Variant::FarCall(_) => todo!(),
                Variant::Ret(_) => {
                    // TODO: This is not how return works. Fix when we have calls between contracts
                    // hooked up.
                    break;
                }
                Variant::UMA(_) => todo!(),
            }
        }
        vm.current_frame.pc += 1;
        vm.decrease_gas(&opcode);
    }
    let final_storage_value = vm
        .storage
        .borrow()
        .read(&(vm.current_frame.address, U256::zero()))
        .unwrap();
    (final_storage_value, vm)
}

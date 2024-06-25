mod address_operands;
mod op_handlers;
mod opcode;
pub mod state;
mod value;

use op_handlers::add::_add;
use op_handlers::context::_aux_mutating0;
use op_handlers::context::_caller;
use op_handlers::context::_code_address;
use op_handlers::context::_ergs_left;
use op_handlers::context::_get_context_u128;
use op_handlers::context::_increment_tx_number;
use op_handlers::context::_meta;
use op_handlers::context::_set_context_u128;
use op_handlers::context::_sp;
use op_handlers::context::_this;
use op_handlers::div::_div;
use op_handlers::mul::_mul;
use op_handlers::sub::_sub;
pub use opcode::Opcode;
use state::VMState;
use u256::U256;
use zkevm_opcode_defs::definitions::synthesize_opcode_decoding_tables;
use zkevm_opcode_defs::ContextOpcode;
use zkevm_opcode_defs::ISAVersion;
use zkevm_opcode_defs::LogOpcode;
use zkevm_opcode_defs::Opcode as Variant;

/// Run a vm program with a clean VM state.
pub fn run_program(bin_path: &str) -> (U256, VMState) {
    let vm = VMState::new(vec![]);
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
                Variant::Context(context_variant) => match context_variant {
                    ContextOpcode::AuxMutating0 => _aux_mutating0(&mut vm, &opcode),
                    ContextOpcode::Caller => _caller(&mut vm, &opcode),
                    ContextOpcode::CodeAddress => _code_address(&mut vm, &opcode),
                    ContextOpcode::ErgsLeft => _ergs_left(&mut vm, &opcode),
                    ContextOpcode::GetContextU128 => _get_context_u128(&mut vm, &opcode),
                    ContextOpcode::IncrementTxNumber => _increment_tx_number(&mut vm, &opcode),
                    ContextOpcode::Meta => _meta(&mut vm, &opcode),
                    ContextOpcode::SetContextU128 => _set_context_u128(&mut vm, &opcode),
                    ContextOpcode::Sp => _sp(&mut vm, &opcode),
                    ContextOpcode::This => _this(&mut vm, &opcode),
                },
                Variant::Shift(_) => todo!(),
                Variant::Binop(_) => todo!(),
                Variant::Ptr(_) => todo!(),
                Variant::NearCall(_) => todo!(),
                Variant::Log(log_variant) => match log_variant {
                    LogOpcode::StorageRead => todo!(),
                    LogOpcode::StorageWrite => {
                        let src0 = vm.get_register(opcode.src0_index);
                        let src1 = vm.get_register(opcode.src1_index);
                        vm.current_frame.storage.insert(src0, src1);
                    }
                    LogOpcode::ToL1Message => todo!(),
                    LogOpcode::Event => todo!(),
                    LogOpcode::PrecompileCall => todo!(),
                    LogOpcode::Decommit => todo!(),
                    LogOpcode::TransientStorageRead => todo!(),
                    LogOpcode::TransientStorageWrite => todo!(),
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
    let final_storage_value = *vm.current_frame.storage.get(&U256::zero()).unwrap();
    (final_storage_value, vm.clone())
}

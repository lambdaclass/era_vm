mod address_operands;
pub mod call_frame;
mod eravm_error;
pub mod heaps;
mod op_handlers;
mod opcode;
pub mod output;
mod ptr_operator;
pub mod state;
pub mod store;
pub mod tracers;
pub mod utils;
pub mod value;
use std::str::FromStr;

use address_operands::{address_operands_read, address_operands_store};
use eravm_error::{EraVmError, HeapError, OpcodeError};
use op_handlers::add::add;
use op_handlers::and::and;
use op_handlers::aux_heap_read::aux_heap_read;
use op_handlers::aux_heap_write::aux_heap_write;
use op_handlers::context::{
    aux_mutating0, caller, code_address, ergs_left, get_context_u128, increment_tx_number, meta,
    set_context_u128, sp, this,
};
use op_handlers::div::div;
use op_handlers::event::event;
use op_handlers::far_call::far_call;
use op_handlers::fat_pointer_read::fat_pointer_read;
use op_handlers::heap_read::heap_read;
use op_handlers::heap_write::heap_write;
use op_handlers::jump::jump;
use op_handlers::log::{
    storage_read, storage_write, transient_storage_read, transient_storage_write,
};
use zkevm_assembly::zkevm_opcode_defs::decoding::{
    EncodingModeProduction, EncodingModeTesting, VmEncodingMode,
};

use op_handlers::mul::mul;
use op_handlers::near_call::near_call;
use op_handlers::or::or;
use op_handlers::precompile_call::precompile_call;
use op_handlers::ptr_add::ptr_add;
use op_handlers::ptr_pack::ptr_pack;
use op_handlers::ptr_shrink::ptr_shrink;
use op_handlers::ptr_sub::ptr_sub;
use op_handlers::ret::{inexplicit_panic, panic_from_far_call, ret};
use op_handlers::shift::rol;
use op_handlers::shift::ror;
use op_handlers::shift::shl;
use op_handlers::shift::shr;
use op_handlers::sub::sub;
use op_handlers::xor::xor;
pub use opcode::Opcode;
use state::VMState;
use store::Storage;
use tracers::tracer::Tracer;
use u256::U256;
use value::{FatPointer, TaggedValue};
use zkevm_assembly::Assembly;
use zkevm_opcode_defs::definitions::synthesize_opcode_decoding_tables;
use zkevm_opcode_defs::BinopOpcode;
use zkevm_opcode_defs::ContextOpcode;
use zkevm_opcode_defs::ISAVersion;
use zkevm_opcode_defs::LogOpcode;
use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::PtrOpcode;
use zkevm_opcode_defs::RetOpcode;
use zkevm_opcode_defs::ShiftOpcode;
use zkevm_opcode_defs::UMAOpcode;

use crate::store::InMemory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutput {
    Ok(Vec<u8>),
    Revert(Vec<u8>),
    Panic,
}

/// Run a vm program with a given bytecode.
pub fn run_program_with_custom_bytecode(
    vm: VMState,
    storage: &mut dyn Storage,
) -> (ExecutionOutput, VMState) {
    run_opcodes(vm, storage)
}

fn run_opcodes(vm: VMState, storage: &mut dyn Storage) -> (ExecutionOutput, VMState) {
    run(vm.clone(), storage, &mut []).unwrap_or((ExecutionOutput::Panic, vm))
}

/// Run a vm program from the given path using a custom state.
/// Returns the value stored at storage with key 0 and the final vm state.
pub fn program_from_file(bin_path: &str) -> Result<Vec<U256>, EraVmError> {
    let program = std::fs::read(bin_path)?;
    let encoded = String::from_utf8(program).map_err(|_| EraVmError::IncorrectBytecodeFormat)?;
    if &encoded[..2] != "0x" {
        return Err(EraVmError::IncorrectBytecodeFormat);
    }
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

pub fn run(
    mut vm: VMState,
    storage: &mut dyn Storage,
    tracers: &mut [Box<&mut dyn Tracer>],
) -> Result<(ExecutionOutput, VMState), EraVmError> {
    let opcode_table = synthesize_opcode_decoding_tables(11, ISAVersion(2));
    loop {
        let opcode = vm.get_opcode(&opcode_table)?;
        for tracer in tracers.iter_mut() {
            tracer.before_execution(&opcode, &mut vm)?;
        }

        if let Some(_err) = vm.decrease_gas(opcode.gas_cost).err() {
            match inexplicit_panic(&mut vm, storage) {
                Ok(false) => continue,
                _ => return Ok((ExecutionOutput::Panic, vm)),
            }
        }

        if vm.can_execute(&opcode)? {
            let result = match opcode.variant {
                Variant::Invalid(_) => Err(OpcodeError::InvalidOpCode.into()),
                Variant::Nop(_) => {
                    address_operands_read(&mut vm, &opcode)?;
                    address_operands_store(&mut vm, &opcode, TaggedValue::new_raw_integer(0.into()))
                }
                Variant::Add(_) => add(&mut vm, &opcode),
                Variant::Sub(_) => sub(&mut vm, &opcode),
                Variant::Jump(_) => jump(&mut vm, &opcode),
                Variant::Mul(_) => mul(&mut vm, &opcode),
                Variant::Div(_) => div(&mut vm, &opcode),
                Variant::Context(context_variant) => match context_variant {
                    ContextOpcode::AuxMutating0 => aux_mutating0(&mut vm, &opcode),
                    ContextOpcode::Caller => caller(&mut vm, &opcode),
                    ContextOpcode::CodeAddress => code_address(&mut vm, &opcode),
                    ContextOpcode::ErgsLeft => ergs_left(&mut vm, &opcode),
                    ContextOpcode::GetContextU128 => get_context_u128(&mut vm, &opcode),
                    ContextOpcode::IncrementTxNumber => increment_tx_number(&mut vm, &opcode),
                    ContextOpcode::Meta => meta(&mut vm, &opcode),
                    ContextOpcode::SetContextU128 => set_context_u128(&mut vm, &opcode),
                    ContextOpcode::Sp => sp(&mut vm, &opcode),
                    ContextOpcode::This => this(&mut vm, &opcode),
                },
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
                Variant::NearCall(_) => near_call(&mut vm, &opcode, storage),
                Variant::Log(log_variant) => match log_variant {
                    LogOpcode::StorageRead => storage_read(&mut vm, &opcode, storage),
                    LogOpcode::StorageWrite => storage_write(&mut vm, &opcode, storage),
                    LogOpcode::ToL1Message => todo!(),
                    LogOpcode::PrecompileCall => precompile_call(&mut vm, &opcode),
                    LogOpcode::Event => event(&mut vm, &opcode),
                    LogOpcode::Decommit => todo!(),
                    LogOpcode::TransientStorageRead => transient_storage_read(&mut vm, &opcode),
                    LogOpcode::TransientStorageWrite => transient_storage_write(&mut vm, &opcode),
                },
                Variant::FarCall(far_call_variant) => {
                    let res = far_call(&mut vm, &opcode, &far_call_variant, storage);
                    if res.is_err() {
                        panic_from_far_call(&mut vm, &opcode)?;
                        continue;
                    }
                    Ok(())
                }
                Variant::Ret(ret_variant) => match ret_variant {
                    RetOpcode::Ok => match ret(&mut vm, &opcode, storage, ret_variant) {
                        Ok(should_break) => {
                            if should_break {
                                let result = retrieve_result(&mut vm)?;
                                return Ok((ExecutionOutput::Ok(result), vm));
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    },
                    RetOpcode::Revert => match ret(&mut vm, &opcode, storage, ret_variant) {
                        Ok(should_break) => {
                            if should_break {
                                let result = retrieve_result(&mut vm)?;
                                return Ok((ExecutionOutput::Revert(result), vm));
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    },
                    RetOpcode::Panic => match ret(&mut vm, &opcode, storage, ret_variant) {
                        Ok(should_break) => {
                            if should_break {
                                return Ok((ExecutionOutput::Panic, vm));
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    },
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
            if let Err(_err) = result {
                match inexplicit_panic(&mut vm, storage) {
                    Ok(false) => continue,
                    _ => return Ok((ExecutionOutput::Panic, vm)),
                }
            }
            set_pc(&mut vm, &opcode)?;
        } else {
            vm.current_frame_mut()?.pc += 1;
        }
    }
}

// Sets the next PC according to the next opcode
fn set_pc(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let current_pc = vm.current_frame()?.pc;

    vm.current_frame_mut()?.pc = match opcode.variant {
        Variant::FarCall(_) => 0,
        Variant::Ret(_) => current_pc,
        Variant::NearCall(_) => current_pc,
        Variant::Jump(_) => current_pc,
        _ => current_pc + 1,
    };

    Ok(())
}

fn retrieve_result(vm: &mut VMState) -> Result<Vec<u8>, EraVmError> {
    let fat_pointer_src0 = FatPointer::decode(vm.get_register(1).value);
    let range = fat_pointer_src0.start..fat_pointer_src0.start + fat_pointer_src0.len;
    let mut result: Vec<u8> = vec![0; range.len()];
    let end: u32 = (range.end).min(
        (vm.heaps
            .get(fat_pointer_src0.page)
            .ok_or(HeapError::ReadOutOfBounds)?
            .len()) as u32,
    );
    for (i, j) in (range.start..end).enumerate() {
        let current_heap = vm
            .heaps
            .get(fat_pointer_src0.page)
            .ok_or(HeapError::ReadOutOfBounds)?;
        result[i] = current_heap.read_byte(j);
    }
    Ok(result)
}
#[test]
fn test_simple_assembly_u128() {
    let asm_text = r#"
    .text
    .file	"add.zasm"
    .globl	__entry
__entry:
.func_begin0:
    add	3, r0, r1
    sstore	r0, r1
    add	r0, r0, r1
    ret
.func_end0:
    .note.GNU-stack
    .rodata
     .text
        pop #10, r2
        push #3, r1
    "#;
    let mut asm = Assembly::try_from(asm_text.to_owned()).unwrap();

    let instructions = asm.opcodes::<16, EncodingModeTesting>().unwrap();

    dbg!(instructions[0]);

    let bytecode = asm
        .compile_to_bytecode_for_mode::<16, EncodingModeTesting>()
        .unwrap();

    let bytecode_flatten = asm
        .compile_to_bytecode_for_mode::<16, EncodingModeTesting>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect::<Vec<u8>>();

    let first_opcode = &bytecode_flatten[0..=15];

    let first_bytes: [u8; 16] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00,
        0x39,
    ];

    let parsed = u128::from_be_bytes(first_bytes);
    let opcode = Opcode::from_raw_opcode_u128(parsed);
    dbg!(opcode);

    let mut as_hex = "0x".to_string();

    for byte in bytecode_flatten {
        as_hex = format!("{}{:X}", as_hex, byte);
    }

    dbg!(&as_hex);

    let bytecode_u256 = bytecode
        .iter()
        .map(|raw_opcode| U256::from_big_endian(raw_opcode))
        .collect::<Vec<_>>();

    let vm = VMState::new(
        bytecode_u256,
        vec![],
        u256::H160::from_str("0x0000000000000000000000000000000000008006").unwrap(),
        u256::H160::from_str("0xdeadbeef01000000000000000000000000000000").unwrap(),
        0_u128.into(),
        [
            1, 0, 6, 79, 187, 234, 99, 109, 5, 67, 106, 79, 246, 117, 247, 249, 19, 126, 0, 67, 71,
            160, 188, 37, 71, 169, 220, 72, 69, 102, 215, 112,
        ],
    );

    let opcode_table = synthesize_opcode_decoding_tables(11, ISAVersion(2));
    let opcode = vm.get_opcode(&opcode_table).unwrap();
    dbg!(opcode);
    // run_program_with_custom_bytecode(vm, &mut InMemory::new_empty());
}

use std::cell::RefCell;
use std::rc::Rc;

use u256::U256;
use zkevm_opcode_defs::{
    synthesize_opcode_decoding_tables, BinopOpcode, ContextOpcode, ISAVersion, LogOpcode,
    PtrOpcode, RetOpcode, ShiftOpcode, UMAOpcode,
};

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::eravm_error::HeapError;
use crate::event;
use crate::meta;
use crate::op_handlers::add::add;
use crate::op_handlers::and::and;
use crate::op_handlers::aux_heap_read::aux_heap_read;
use crate::op_handlers::aux_heap_write::aux_heap_write;
use crate::op_handlers::context::{
    aux_mutating0, caller, code_address, ergs_left, get_context_u128, increment_tx_number,
    set_context_u128, sp, this,
};
use crate::op_handlers::div::div;
use crate::op_handlers::far_call::far_call;
use crate::op_handlers::fat_pointer_read::fat_pointer_read;
use crate::op_handlers::heap_read::heap_read;
use crate::op_handlers::heap_write::heap_write;
use crate::op_handlers::jump::jump;
use crate::op_handlers::log::{
    storage_read, storage_write, transient_storage_read, transient_storage_write,
};
use crate::op_handlers::mul::mul;
use crate::op_handlers::near_call::near_call;
use crate::op_handlers::ok::ok;
use crate::op_handlers::or::or;
use crate::op_handlers::precompile_call::precompile_call;
use crate::op_handlers::ptr_add::ptr_add;
use crate::op_handlers::ptr_pack::ptr_pack;
use crate::op_handlers::ptr_shrink::ptr_shrink;
use crate::op_handlers::ptr_sub::ptr_sub;
use crate::op_handlers::revert::{handle_error, revert};
use crate::op_handlers::shift::{rol, ror, shl, shr};
use crate::op_handlers::sub::sub;
use crate::op_handlers::xor::xor;
use crate::panic;
use crate::value::{FatPointer, TaggedValue};
use crate::{
    eravm_error::EraVmError, op_handlers::revert::revert_out_of_gas, store::Storage,
    tracers::tracer::Tracer, VMState,
};
use crate::{Opcode, Variant};

#[derive(Debug)]
pub struct LambdaVm {
    pub state: VMState,
    pub storage: Rc<RefCell<dyn Storage>>,
}

impl LambdaVm {
    pub fn new(state: VMState, storage: Rc<RefCell<dyn Storage>>) -> Self {
        Self { state, storage }
    }

    /// Run a vm program with a given bytecode.
    pub fn run_program_with_custom_bytecode(&mut self) -> (ExecutionOutput, VMState) {
        self.run_opcodes()
    }

    fn run_opcodes(&mut self) -> (ExecutionOutput, VMState) {
        self.run(&mut [])
            .unwrap_or((ExecutionOutput::Panic, self.state.clone()))
    }

    /// Run a vm program from the given path using a custom state.
    /// Returns the value stored at storage with key 0 and the final vm state.
    pub fn program_from_file(&self, bin_path: &str) -> Result<Vec<U256>, EraVmError> {
        let program = std::fs::read(bin_path)?;
        let encoded =
            String::from_utf8(program).map_err(|_| EraVmError::IncorrectBytecodeFormat)?;
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

    /// Run a vm program with a clean VM state.
    pub fn run_program(
        &mut self,
        bin_path: &str,
        tracers: &mut [Box<&mut dyn Tracer>],
    ) -> ExecutionOutput {
        match self.run_program_with_error(bin_path, tracers) {
            Ok((execution_output, _vm)) => execution_output,
            Err(_) => ExecutionOutput::Panic, // TODO: fix this
        }
    }

    pub fn run_program_with_error(
        &mut self,
        bin_path: &str,
        tracers: &mut [Box<&mut dyn Tracer>],
    ) -> Result<(ExecutionOutput, VMState), EraVmError> {
        let program_code = self.program_from_file(bin_path)?;
        self.state.load_program(program_code);
        self.run(tracers)
    }

    pub fn run(
        &mut self,
        tracers: &mut [Box<&mut dyn Tracer>],
    ) -> Result<(ExecutionOutput, VMState), EraVmError> {
        let opcode_table = synthesize_opcode_decoding_tables(11, ISAVersion(2));
        loop {
            let opcode = self.state.get_opcode(&opcode_table)?;
            for tracer in tracers.iter_mut() {
                tracer.before_execution(&opcode, &mut self.state)?;
            }

            let out_of_gas = self.state.decrease_gas(opcode.gas_cost)?;
            if out_of_gas {
                revert_out_of_gas(&mut self.state)?;
            }

            if self.state.predicate_holds(&opcode.predicate) {
                let result = match opcode.variant {
                    Variant::Invalid(_) => todo!(),
                    Variant::Nop(_) => {
                        address_operands_read(&mut self.state, &opcode)?;
                        address_operands_store(
                            &mut self.state,
                            &opcode,
                            TaggedValue::new_raw_integer(0.into()),
                        )
                    }
                    Variant::Add(_) => add(&mut self.state, &opcode),
                    Variant::Sub(_) => sub(&mut self.state, &opcode),
                    Variant::Jump(_) => jump(&mut self.state, &opcode),
                    Variant::Mul(_) => mul(&mut self.state, &opcode),
                    Variant::Div(_) => div(&mut self.state, &opcode),
                    Variant::Context(context_variant) => match context_variant {
                        ContextOpcode::AuxMutating0 => aux_mutating0(&mut self.state, &opcode),
                        ContextOpcode::Caller => caller(&mut self.state, &opcode),
                        ContextOpcode::CodeAddress => code_address(&mut self.state, &opcode),
                        ContextOpcode::ErgsLeft => ergs_left(&mut self.state, &opcode),
                        ContextOpcode::GetContextU128 => get_context_u128(&mut self.state, &opcode),
                        ContextOpcode::IncrementTxNumber => {
                            increment_tx_number(&mut self.state, &opcode)
                        }
                        ContextOpcode::Meta => meta(&mut self.state, &opcode),
                        ContextOpcode::SetContextU128 => set_context_u128(&mut self.state, &opcode),
                        ContextOpcode::Sp => sp(&mut self.state, &opcode),
                        ContextOpcode::This => this(&mut self.state, &opcode),
                    },
                    Variant::Shift(shift_variant) => match shift_variant {
                        ShiftOpcode::Shl => shl(&mut self.state, &opcode),
                        ShiftOpcode::Shr => shr(&mut self.state, &opcode),
                        ShiftOpcode::Rol => rol(&mut self.state, &opcode),
                        ShiftOpcode::Ror => ror(&mut self.state, &opcode),
                    },
                    Variant::Binop(binop) => match binop {
                        BinopOpcode::Xor => xor(&mut self.state, &opcode),
                        BinopOpcode::And => and(&mut self.state, &opcode),
                        BinopOpcode::Or => or(&mut self.state, &opcode),
                    },
                    Variant::Ptr(ptr_variant) => match ptr_variant {
                        PtrOpcode::Add => ptr_add(&mut self.state, &opcode),
                        PtrOpcode::Sub => ptr_sub(&mut self.state, &opcode),
                        PtrOpcode::Pack => ptr_pack(&mut self.state, &opcode),
                        PtrOpcode::Shrink => ptr_shrink(&mut self.state, &opcode),
                    },
                    Variant::NearCall(_) => near_call(&mut self.state, &opcode),
                    Variant::Log(log_variant) => match log_variant {
                        LogOpcode::StorageRead => {
                            storage_read(&mut self.state, &opcode, &mut *self.storage.borrow_mut())
                        }
                        LogOpcode::StorageWrite => {
                            storage_write(&mut self.state, &opcode, &mut *self.storage.borrow_mut())
                        }
                        LogOpcode::ToL1Message => todo!(),
                        LogOpcode::PrecompileCall => precompile_call(&mut self.state, &opcode),
                        LogOpcode::Event => event(&mut self.state, &opcode),
                        LogOpcode::Decommit => todo!(),
                        LogOpcode::TransientStorageRead => {
                            transient_storage_read(&mut self.state, &opcode)
                        }
                        LogOpcode::TransientStorageWrite => {
                            transient_storage_write(&mut self.state, &opcode)
                        }
                    },
                    Variant::FarCall(far_call_variant) => far_call(
                        &mut self.state,
                        &opcode,
                        &far_call_variant,
                        &mut *self.storage.borrow_mut(),
                    ),
                    // TODO: This is not how return works. Fix when we have calls between contracts
                    // hooked up.
                    // This is only to keep the context for tests
                    Variant::Ret(ret_variant) => match ret_variant {
                        RetOpcode::Ok => match ok(&mut self.state, &opcode) {
                            Ok(should_break) => {
                                if should_break {
                                    break;
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        RetOpcode::Revert => match revert(&mut self.state, &opcode) {
                            Ok(should_break) => {
                                if should_break {
                                    return Ok((
                                        ExecutionOutput::Revert(vec![]),
                                        self.state.clone(),
                                    ));
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        RetOpcode::Panic => match panic(&mut self.state, &opcode) {
                            Ok(should_break) => {
                                if should_break {
                                    return Ok((ExecutionOutput::Panic, self.state.clone()));
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                    },
                    Variant::UMA(uma_variant) => match uma_variant {
                        UMAOpcode::HeapRead => heap_read(&mut self.state, &opcode),
                        UMAOpcode::HeapWrite => heap_write(&mut self.state, &opcode),
                        UMAOpcode::AuxHeapRead => aux_heap_read(&mut self.state, &opcode),
                        UMAOpcode::AuxHeapWrite => aux_heap_write(&mut self.state, &opcode),
                        UMAOpcode::FatPointerRead => fat_pointer_read(&mut self.state, &opcode),
                        UMAOpcode::StaticMemoryRead => todo!(),
                        UMAOpcode::StaticMemoryWrite => todo!(),
                    },
                };
                if let Err(e) = result {
                    handle_error(&mut self.state, e)?;
                }
            }
            self.state.current_frame_mut()?.pc =
                opcode_pc_set(&opcode, self.state.current_frame()?.pc);
        }
        let fat_pointer_src0 = FatPointer::decode(self.state.get_register(1).value);
        let range = fat_pointer_src0.start..fat_pointer_src0.start + fat_pointer_src0.len;
        let mut result: Vec<u8> = vec![0; range.len()];
        let end: u32 = (range.end).min(
            (self
                .state
                .heaps
                .get(fat_pointer_src0.page)
                .ok_or(HeapError::ReadOutOfBounds)?
                .len()) as u32,
        );
        for (i, j) in (range.start..end).enumerate() {
            let current_heap = self
                .state
                .heaps
                .get(fat_pointer_src0.page)
                .ok_or(HeapError::ReadOutOfBounds)?;
            result[i] = current_heap.read_byte(j);
        }
        Ok((ExecutionOutput::Ok(result), self.state.clone()))
    }
}

// Set the next PC according to th enext opcode
fn opcode_pc_set(opcode: &Opcode, current_pc: u64) -> u64 {
    match opcode.variant {
        Variant::FarCall(_) => 0,
        _ => current_pc + 1,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutput {
    Ok(Vec<u8>),
    Revert(Vec<u8>),
    Panic,
}

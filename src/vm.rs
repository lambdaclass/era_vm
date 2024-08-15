use std::cell::RefCell;
use std::rc::Rc;

use u256::U256;
use zkevm_opcode_defs::{
    BinopOpcode, ContextOpcode, LogOpcode, PtrOpcode, RetOpcode, ShiftOpcode, UMAOpcode,
};

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::eravm_error::{HeapError, OpcodeError};
use crate::op_handlers::add::add;
use crate::op_handlers::and::and;
use crate::op_handlers::aux_heap_read::aux_heap_read;
use crate::op_handlers::aux_heap_write::aux_heap_write;
use crate::op_handlers::context::{
    caller, code_address, ergs_left, get_context_u128, increment_tx_number, meta, set_context_u128,
    sp, this,
};
use crate::op_handlers::div::div;
use crate::op_handlers::event::event;
use crate::op_handlers::far_call::far_call;
use crate::op_handlers::fat_pointer_read::fat_pointer_read;
use crate::op_handlers::heap_read::heap_read;
use crate::op_handlers::heap_write::heap_write;
use crate::op_handlers::jump::jump;
use crate::op_handlers::log::{
    add_l2_to_l1_message, storage_read, storage_write, transient_storage_read,
    transient_storage_write,
};
use crate::op_handlers::mul::mul;
use crate::op_handlers::near_call::near_call;
use crate::op_handlers::opcode_decommit::opcode_decommit;
use crate::op_handlers::or::or;
use crate::op_handlers::precompile_call::precompile_call;
use crate::op_handlers::ptr_add::ptr_add;
use crate::op_handlers::ptr_pack::ptr_pack;
use crate::op_handlers::ptr_shrink::ptr_shrink;
use crate::op_handlers::ptr_sub::ptr_sub;
use crate::op_handlers::ret::{inexplicit_panic, panic_from_far_call, ret};
use crate::op_handlers::shift::{rol, ror, shl, shr};
use crate::op_handlers::sub::sub;
use crate::op_handlers::unimplemented::unimplemented;
use crate::op_handlers::xor::xor;
use crate::state::VMState;
use crate::store::{ContractStorage, InitialStorage};
use crate::tracers::blob_saver_tracer::BlobSaverTracer;
use crate::value::{FatPointer, TaggedValue};
use crate::{eravm_error::EraVmError, tracers::tracer::Tracer, Execution};
use crate::{Opcode, Variant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutput {
    Ok(Vec<u8>),
    Revert(Vec<u8>),
    Panic,
    SuspendedOnHook { hook: u32, pc_to_resume_from: u16 },
}

#[derive(Debug)]
pub struct EraVM {
    pub state: VMState,
    pub execution: Execution,
}

pub enum EncodingMode {
    Production,
    Testing,
}

impl EraVM {
    pub fn new(
        execution: Execution,
        initial_storage: Rc<RefCell<dyn InitialStorage>>,
        contract_storage: Rc<RefCell<dyn ContractStorage>>,
    ) -> Self {
        Self {
            state: VMState::new(initial_storage, contract_storage),
            execution,
        }
    }

    /// Run a vm program with a given bytecode.
    pub fn run_program_with_custom_bytecode(&mut self) -> (ExecutionOutput, BlobSaverTracer) {
        self.run_opcodes()
    }

    pub fn run_program_with_test_encode(&mut self) -> (ExecutionOutput, BlobSaverTracer) {
        let mut tracer = BlobSaverTracer::new();
        let r = self
            .run(&mut [Box::new(&mut tracer)], EncodingMode::Testing)
            .unwrap_or(ExecutionOutput::Panic);

        (r, tracer)
    }

    fn run_opcodes(&mut self) -> (ExecutionOutput, BlobSaverTracer) {
        let mut tracer = BlobSaverTracer::new();
        let r = self
            .run(&mut [Box::new(&mut tracer)], EncodingMode::Production)
            .unwrap_or(ExecutionOutput::Panic);

        (r, tracer)
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

    #[allow(non_upper_case_globals)]
    pub fn run(
        &mut self,
        tracers: &mut [Box<&mut dyn Tracer>],
        enc_mode: EncodingMode,
    ) -> Result<ExecutionOutput, EraVmError> {
        loop {
            let opcode = match enc_mode {
                EncodingMode::Testing => self.execution.get_opcode_with_test_encode()?,
                EncodingMode::Production => self.execution.get_opcode()?,
            };
            for tracer in tracers.iter_mut() {
                tracer.before_execution(&opcode, &mut self.execution)?;
            }

            let can_execute = self.execution.can_execute(&opcode);

            if self.execution.decrease_gas(opcode.gas_cost).is_err() || can_execute.is_err() {
                match inexplicit_panic(&mut self.execution, &mut self.state) {
                    Ok(false) => continue,
                    _ => return Ok(ExecutionOutput::Panic),
                }
            }

            if can_execute? {
                let result = match opcode.variant {
                    Variant::Invalid(_) => Err(OpcodeError::InvalidOpCode.into()),
                    Variant::Nop(_) => {
                        address_operands_read(&mut self.execution, &opcode)?;
                        address_operands_store(
                            &mut self.execution,
                            &opcode,
                            TaggedValue::new_raw_integer(0.into()),
                        )
                    }
                    Variant::Add(_) => add(&mut self.execution, &opcode),
                    Variant::Sub(_) => sub(&mut self.execution, &opcode),
                    Variant::Jump(_) => jump(&mut self.execution, &opcode),
                    Variant::Mul(_) => mul(&mut self.execution, &opcode),
                    Variant::Div(_) => div(&mut self.execution, &opcode),
                    Variant::Context(context_variant) => match context_variant {
                        ContextOpcode::AuxMutating0 => unimplemented(&mut self.execution, &opcode),
                        ContextOpcode::Caller => caller(&mut self.execution, &opcode),
                        ContextOpcode::CodeAddress => code_address(&mut self.execution, &opcode),
                        ContextOpcode::ErgsLeft => ergs_left(&mut self.execution, &opcode),
                        ContextOpcode::GetContextU128 => {
                            get_context_u128(&mut self.execution, &opcode)
                        }
                        ContextOpcode::IncrementTxNumber => {
                            increment_tx_number(&mut self.execution, &opcode)
                        }
                        ContextOpcode::Meta => meta(&mut self.execution, &opcode),
                        ContextOpcode::SetContextU128 => {
                            set_context_u128(&mut self.execution, &opcode)
                        }
                        ContextOpcode::Sp => sp(&mut self.execution, &opcode),
                        ContextOpcode::This => this(&mut self.execution, &opcode),
                    },
                    Variant::Shift(shift_variant) => match shift_variant {
                        ShiftOpcode::Shl => shl(&mut self.execution, &opcode),
                        ShiftOpcode::Shr => shr(&mut self.execution, &opcode),
                        ShiftOpcode::Rol => rol(&mut self.execution, &opcode),
                        ShiftOpcode::Ror => ror(&mut self.execution, &opcode),
                    },
                    Variant::Binop(binop) => match binop {
                        BinopOpcode::Xor => xor(&mut self.execution, &opcode),
                        BinopOpcode::And => and(&mut self.execution, &opcode),
                        BinopOpcode::Or => or(&mut self.execution, &opcode),
                    },
                    Variant::Ptr(ptr_variant) => match ptr_variant {
                        PtrOpcode::Add => ptr_add(&mut self.execution, &opcode),
                        PtrOpcode::Sub => ptr_sub(&mut self.execution, &opcode),
                        PtrOpcode::Pack => ptr_pack(&mut self.execution, &opcode),
                        PtrOpcode::Shrink => ptr_shrink(&mut self.execution, &opcode),
                    },
                    Variant::NearCall(_) => near_call(&mut self.execution, &opcode, &self.state),
                    Variant::Log(log_variant) => match log_variant {
                        LogOpcode::StorageRead => {
                            storage_read(&mut self.execution, &opcode, &self.state)
                        }
                        LogOpcode::StorageWrite => {
                            storage_write(&mut self.execution, &opcode, &mut self.state)
                        }
                        LogOpcode::ToL1Message => {
                            add_l2_to_l1_message(&mut self.execution, &opcode, &mut self.state)
                        }
                        LogOpcode::PrecompileCall => precompile_call(&mut self.execution, &opcode),
                        LogOpcode::Event => event(&mut self.execution, &opcode, &mut self.state),
                        LogOpcode::Decommit => {
                            opcode_decommit(&mut self.execution, &opcode, &mut self.state)
                        }
                        LogOpcode::TransientStorageRead => {
                            transient_storage_read(&mut self.execution, &opcode, &self.state)
                        }
                        LogOpcode::TransientStorageWrite => {
                            transient_storage_write(&mut self.execution, &opcode, &mut self.state)
                        }
                    },
                    Variant::FarCall(far_call_variant) => {
                        let res = far_call(
                            &mut self.execution,
                            &opcode,
                            &far_call_variant,
                            &mut self.state,
                        );
                        if res.is_err() {
                            panic_from_far_call(&mut self.execution, &opcode)?;
                            continue;
                        }
                        Ok(())
                    }
                    Variant::Ret(ret_variant) => match ret_variant {
                        RetOpcode::Ok => {
                            match ret(&mut self.execution, &opcode, &mut self.state, ret_variant) {
                                Ok(should_break) => {
                                    if should_break {
                                        let result = retrieve_result(&mut self.execution)?;
                                        return Ok(ExecutionOutput::Ok(result));
                                    }
                                    Ok(())
                                }
                                Err(e) => Err(e),
                            }
                        }
                        RetOpcode::Revert => {
                            match ret(&mut self.execution, &opcode, &mut self.state, ret_variant) {
                                Ok(should_break) => {
                                    if should_break {
                                        let result = retrieve_result(&mut self.execution)?;
                                        return Ok(ExecutionOutput::Revert(result));
                                    }
                                    Ok(())
                                }
                                Err(e) => Err(e),
                            }
                        }
                        RetOpcode::Panic => {
                            match ret(&mut self.execution, &opcode, &mut self.state, ret_variant) {
                                Ok(should_break) => {
                                    if should_break {
                                        return Ok(ExecutionOutput::Panic);
                                    }
                                    Ok(())
                                }
                                Err(e) => Err(e),
                            }
                        }
                    },
                    Variant::UMA(uma_variant) => match uma_variant {
                        UMAOpcode::HeapRead => heap_read(&mut self.execution, &opcode),
                        UMAOpcode::HeapWrite => {
                            let result = heap_write(&mut self.execution, &opcode);
                            match result {
                                exec_hook @ Ok(ExecutionOutput::SuspendedOnHook { .. }) => {
                                    return exec_hook
                                }
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }

                        UMAOpcode::AuxHeapRead => aux_heap_read(&mut self.execution, &opcode),
                        UMAOpcode::AuxHeapWrite => aux_heap_write(&mut self.execution, &opcode),
                        UMAOpcode::FatPointerRead => fat_pointer_read(&mut self.execution, &opcode),
                        UMAOpcode::StaticMemoryRead => unimplemented(&mut self.execution, &opcode),
                        UMAOpcode::StaticMemoryWrite => unimplemented(&mut self.execution, &opcode),
                    },
                };
                if let Err(err) = result {
                    if let EraVmError::OpcodeError(OpcodeError::UnimplementedOpcode) = err {
                        return Ok(ExecutionOutput::Panic);
                    }

                    match inexplicit_panic(&mut self.execution, &mut self.state) {
                        Ok(false) => continue,
                        _ => return Ok(ExecutionOutput::Panic),
                    }
                }
                set_pc(&mut self.execution, &opcode)?;
            } else {
                self.execution.current_frame_mut()?.pc += 1;
            }
        }
    }
}

// Sets the next PC according to the next opcode
fn set_pc(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
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

fn retrieve_result(vm: &mut Execution) -> Result<Vec<u8>, EraVmError> {
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

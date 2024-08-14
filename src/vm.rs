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
use crate::store::{ContractStorage, InitialStorage, StateStorage};
use crate::tracers::blob_saver_tracer::BlobSaverTracer;
use crate::value::{FatPointer, TaggedValue};
use crate::{eravm_error::EraVmError, tracers::tracer::Tracer, VMState};
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
    pub contract_storage: Rc<RefCell<dyn ContractStorage>>,
    pub state_storage: StateStorage,
    pub transient_storage: StateStorage,
    pub pubdata: i32,
    pub pubdata_costs: Vec<i32>,
}

pub enum EncodingMode {
    Production,
    Testing,
}

impl EraVM {
    pub fn new(
        state: VMState,
        initial_storage: Rc<RefCell<dyn InitialStorage>>,
        contract_storage: Rc<RefCell<dyn ContractStorage>>,
    ) -> Self {
        Self {
            state,
            contract_storage,
            state_storage: StateStorage::new(initial_storage),
            transient_storage: StateStorage::default(),
            pubdata: 0,
            pubdata_costs: vec![],
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
                EncodingMode::Testing => self.state.get_opcode_with_test_encode()?,
                EncodingMode::Production => self.state.get_opcode()?,
            };
            for tracer in tracers.iter_mut() {
                tracer.before_execution(&opcode, &mut self.state)?;
            }

            let can_execute = self.state.can_execute(&opcode);

            if self.state.decrease_gas(opcode.gas_cost).is_err() || can_execute.is_err() {
                match inexplicit_panic(
                    &mut self.state,
                    &mut self.pubdata,
                    &mut self.pubdata_costs,
                    &mut self.state_storage,
                    &mut self.transient_storage,
                ) {
                    Ok(false) => continue,
                    _ => return Ok(ExecutionOutput::Panic),
                }
            }

            if can_execute? {
                let result = match opcode.variant {
                    Variant::Invalid(_) => Err(OpcodeError::InvalidOpCode.into()),
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
                        ContextOpcode::AuxMutating0 => unimplemented(&mut self.state, &opcode),
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
                    Variant::NearCall(_) => near_call(
                        &mut self.state,
                        &opcode,
                        &mut self.pubdata,
                        &mut self.pubdata_costs,
                        &self.state_storage,
                        &self.transient_storage,
                    ),
                    Variant::Log(log_variant) => match log_variant {
                        LogOpcode::StorageRead => {
                            storage_read(&mut self.state, &opcode, &self.state_storage)
                        }
                        LogOpcode::StorageWrite => {
                            storage_write(&mut self.state, &opcode, &mut self.state_storage)
                        }
                        LogOpcode::ToL1Message => {
                            add_l2_to_l1_message(&mut self.state, &opcode, &mut self.state_storage)
                        }
                        LogOpcode::PrecompileCall => precompile_call(&mut self.state, &opcode),
                        LogOpcode::Event => event(&mut self.state, &opcode),
                        LogOpcode::Decommit => opcode_decommit(
                            &mut self.state,
                            &opcode,
                            &mut *self.contract_storage.borrow_mut(),
                        ),
                        LogOpcode::TransientStorageRead => transient_storage_read(
                            &mut self.state,
                            &opcode,
                            &self.transient_storage,
                        ),
                        LogOpcode::TransientStorageWrite => transient_storage_write(
                            &mut self.state,
                            &opcode,
                            &mut self.transient_storage,
                        ),
                    },
                    Variant::FarCall(far_call_variant) => {
                        let res = far_call(
                            &mut self.state,
                            &opcode,
                            &far_call_variant,
                            &mut self.pubdata,
                            &mut self.pubdata_costs,
                            &mut self.state_storage,
                            &mut *self.contract_storage.borrow_mut(),
                            &self.transient_storage,
                        );
                        if res.is_err() {
                            panic_from_far_call(&mut self.state, &opcode)?;
                            continue;
                        }
                        Ok(())
                    }
                    Variant::Ret(ret_variant) => match ret_variant {
                        RetOpcode::Ok => match ret(
                            &mut self.state,
                            &opcode,
                            &mut self.pubdata,
                            &mut self.pubdata_costs,
                            &mut self.state_storage,
                            &mut self.transient_storage,
                            ret_variant,
                        ) {
                            Ok(should_break) => {
                                if should_break {
                                    let result = retrieve_result(&mut self.state)?;
                                    return Ok(ExecutionOutput::Ok(result));
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        RetOpcode::Revert => match ret(
                            &mut self.state,
                            &opcode,
                            &mut self.pubdata,
                            &mut self.pubdata_costs,
                            &mut self.state_storage,
                            &mut self.transient_storage,
                            ret_variant,
                        ) {
                            Ok(should_break) => {
                                if should_break {
                                    let result = retrieve_result(&mut self.state)?;
                                    return Ok(ExecutionOutput::Revert(result));
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        RetOpcode::Panic => match ret(
                            &mut self.state,
                            &opcode,
                            &mut self.pubdata,
                            &mut self.pubdata_costs,
                            &mut self.state_storage,
                            &mut self.transient_storage,
                            ret_variant,
                        ) {
                            Ok(should_break) => {
                                if should_break {
                                    return Ok(ExecutionOutput::Panic);
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                    },
                    Variant::UMA(uma_variant) => match uma_variant {
                        UMAOpcode::HeapRead => heap_read(&mut self.state, &opcode),
                        UMAOpcode::HeapWrite => {
                            let result = heap_write(&mut self.state, &opcode);
                            match result {
                                exec_hook @ Ok(ExecutionOutput::SuspendedOnHook { .. }) => {
                                    return exec_hook
                                }
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }

                        UMAOpcode::AuxHeapRead => aux_heap_read(&mut self.state, &opcode),
                        UMAOpcode::AuxHeapWrite => aux_heap_write(&mut self.state, &opcode),
                        UMAOpcode::FatPointerRead => fat_pointer_read(&mut self.state, &opcode),
                        UMAOpcode::StaticMemoryRead => unimplemented(&mut self.state, &opcode),
                        UMAOpcode::StaticMemoryWrite => unimplemented(&mut self.state, &opcode),
                    },
                };
                if let Err(err) = result {
                    if let EraVmError::OpcodeError(OpcodeError::UnimplementedOpcode) = err {
                        return Ok(ExecutionOutput::Panic);
                    }

                    match inexplicit_panic(
                        &mut self.state,
                        &mut self.pubdata,
                        &mut self.pubdata_costs,
                        &mut self.state_storage,
                        &mut self.transient_storage,
                    ) {
                        Ok(false) => continue,
                        _ => return Ok(ExecutionOutput::Panic),
                    }
                }
                set_pc(&mut self.state, &opcode)?;
            } else {
                self.state.current_frame_mut()?.pc += 1;
            }
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

use u256::U256;

use crate::call_frame::CallFrame;
use crate::eravm_error::EraVmError;
use crate::{opcode::Opcode, state::VMState};

pub fn near_call(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let abi_reg = vm.get_register(opcode.src0_index);
    let call_pc = (opcode.imm0 - 1) as u64;
    let exception_handler = opcode.imm1;

    let ergs_passed = NearCallABI::new(abi_reg.value).ergs_passed;

    let (callee_ergs, caller_ergs) = split_ergs_caller_calee(ergs_passed, vm.gas_left()?);

    vm.set_gas_left(caller_ergs)?;

    vm.flag_eq = false;
    vm.flag_gt = false;
    vm.flag_lt_of = false;

    let current_frame = vm.current_frame_mut()?;

    current_frame.pc += 1; // The +1 used later will actually increase the pc of the new frame
    let new_sp = current_frame.sp;

    // Create new frame
    let new_frame =
        CallFrame::new_near_call_frame(new_sp, call_pc, callee_ergs, exception_handler as u64);

    vm.push_near_call_frame(new_frame)
}

fn split_ergs_caller_calee(ergs_passed: u32, caller_ergs: u32) -> (u32, u32) {
    if ergs_passed == 0 {
        return (caller_ergs, 0);
    }
    if caller_ergs >= ergs_passed {
        return (ergs_passed, caller_ergs - ergs_passed);
    }
    (caller_ergs, 0)
}
struct NearCallABI {
    ergs_passed: u32,
}

impl NearCallABI {
    fn new(ergs_passed: U256) -> Self {
        Self {
            ergs_passed: ergs_passed.low_u32(),
        }
    }
}

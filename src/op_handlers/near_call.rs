use std::cell::RefCell;
use std::rc::Rc;

use u256::U256;

use crate::call_frame::CallFrame;
use crate::store::Storage;
use crate::{opcode::Opcode, state::VMState};

pub fn _near_call(vm: &mut VMState, opcode: &Opcode) {
    let abi_reg = vm.get_register(opcode.src0_index);
    let callee_address = opcode.imm0;
    let exception_handler = opcode.imm1;

    let ergs_passed = NearCallABI::new(abi_reg.value).ergs_passed;

    let (callee_ergs, caller_ergs) = split_ergs_caller_calee(ergs_passed, vm.gas_left());

    vm.set_gas_left(caller_ergs);

    vm.flag_eq = false;
    vm.flag_gt = false;
    vm.flag_lt_of = false;

    let current_frame = vm.current_frame_mut();

    current_frame.pc += 1; // The +1 used later will actually increase the pc of the new frame
    let new_stack = current_frame.stack.clone();
    let new_heap = current_frame.heap.clone();
    let new_aux_heap = current_frame.aux_heap.clone();
    let new_storage = Rc::new(RefCell::new((*current_frame.storage.borrow()).fake_clone())); // TODO: Implement proper rollback
    let new_code_page = current_frame.code_page.clone();
    let new_transient_storage = current_frame.transient_storage.fake_clone();

    // Create new frame
    let new_frame = CallFrame::new_near_call_frame(
        new_stack,
        new_heap,
        new_aux_heap,
        new_code_page,
        callee_address as u64 - 1,
        new_storage,
        callee_ergs,
        new_transient_storage,
        exception_handler as u64,
    );

    vm.push_near_call_frame(new_frame);
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

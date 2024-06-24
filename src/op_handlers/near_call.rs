use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

pub fn _near_call(vm: &mut VMState, opcode: &Opcode) {
    let abi_reg = vm.get_register(opcode.src0_index);
    let calle_address = opcode.imm0;
    let exception_handler = opcode.imm1;

    let ergs_passed = NearCallABI::new(abi_reg).ergs_passed;

    let (callee_ergs, caller_ergs) = split_ergs_caller_calee(ergs_passed, vm.gas_left());

    vm.set_gas_left(caller_ergs);
    
    vm.flag_eq = false;
    vm.flag_gt = false;
    vm.flag_lt_of = false;

    vm.current_frame.pc += 1; // The +1 used later will actually increase the pc of the new frame
    let new_stack = vm.current_frame.stack.clone();

    // Create new context

    vm.current_frame.pc -= 1; // We account for the +1 done at the end

}

fn split_ergs_caller_calee(ergs_passed: u32, caller_ergs: u32) -> (u32, u32) {
    if ergs_passed == 0 {
        return (caller_ergs, 0);
    }
    if caller_ergs >= ergs_passed {
        return (caller_ergs - ergs_passed, ergs_passed);
    }
    (caller_ergs, 0)

}
struct NearCallABI {
    ergs_passed: u32,
}

impl NearCallABI {
    fn new(ergs_passed: U256) -> Self {
        Self { ergs_passed: ergs_passed.low_u32() }
    }
}

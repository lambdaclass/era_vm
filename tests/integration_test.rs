use era_vm::{
    run_program, run_program_with_custom_state,
    state::VMStateBuilder,
    value::{FatPointer, TaggedValue},
};
use std::time::{SystemTime, UNIX_EPOCH};
use u256::U256;
const ARTIFACTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/program_artifacts");

// I don't want to add another crate just yet, so I'll use this to test below.
fn fake_rand() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize
}
fn make_bin_path_yul(file_name: &str) -> String {
    format!(
        "{}/{}.artifacts.yul/programs/{}.yul.zbin",
        ARTIFACTS_PATH, file_name, file_name
    )
}

fn make_bin_path_asm(file_name: &str) -> String {
    format!(
        "{}/{}.artifacts.zasm/programs/{}.zasm.zbin",
        ARTIFACTS_PATH, file_name, file_name
    )
}

#[test]
fn test_add_yul() {
    let bin_path = make_bin_path_yul("add");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
fn test_add_asm() {
    let bin_path = make_bin_path_asm("add");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
fn test_add_registers() {
    let bin_path = make_bin_path_asm("add_registers");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
fn test_add_stack_with_push() {
    let bin_path = make_bin_path_asm("add_stack_with_push");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
#[should_panic]
fn test_add_stack_out_of_bounds() {
    let bin_path = make_bin_path_asm("add_stack_out_of_bounds");
    run_program(&bin_path);
}

#[test]
fn test_sub_asm_simple() {
    let bin_path = make_bin_path_asm("sub_simple");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
fn test_sub_asm() {
    let bin_path = make_bin_path_asm("sub_should_be_zero");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_add_stack_with_pop() {
    let bin_path = make_bin_path_asm("add_stack_with_pop");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("2").unwrap());
}

#[test]
#[should_panic]
fn test_add_stack_with_pop_out_of_bounds() {
    let bin_path = make_bin_path_asm("add_stack_with_pop_out_of_bounds");
    run_program(&bin_path);
}

#[test]
fn test_add_code_page() {
    let bin_path = make_bin_path_asm("add_code_page");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("42").unwrap());
}

#[test]
fn test_add_does_not_run_if_eq_is_not_set() {
    let bin_path = make_bin_path_asm("add_conditional");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_add_runs_if_eq_is_set() {
    let bin_path = make_bin_path_asm("add_conditional_eq");
    let vm_with_custom_flags = VMStateBuilder::new().eq_flag(true).build();
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

#[test]
fn test_add_does_run_if_lt_is_set() {
    let bin_path = make_bin_path_asm("add_conditional_lt");
    let vm_with_custom_flags = VMStateBuilder::new().lt_of_flag(true).build();
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

#[test]
fn test_add_does_not_run_if_lt_is_not_set() {
    let bin_path = make_bin_path_asm("add_conditional_not_lt");
    let vm_with_custom_flags = VMStateBuilder::new()
        .lt_of_flag(true)
        .eq_flag(false)
        .gt_flag(true)
        .build();
    // VMState::new_with_flag_state(true, false, true);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

#[test]
fn test_add_does_run_if_gt_is_set() {
    let bin_path = make_bin_path_asm("add_conditional_gt");
    let vm_with_custom_flags = VMStateBuilder::new()
        .lt_of_flag(true)
        .eq_flag(false)
        .gt_flag(true)
        .build();
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("20").unwrap());
}

#[test]
fn test_add_does_not_run_if_gt_is_not_set() {
    let bin_path = make_bin_path_asm("add_conditional_not_gt");
    let vm_with_custom_flags = VMStateBuilder::new()
        .lt_of_flag(false)
        .eq_flag(false)
        .gt_flag(false)
        .build();
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_add_sets_overflow_flag() {
    let bin_path = make_bin_path_asm("add_sets_overflow");
    let r1 = TaggedValue::new_raw_integer(U256::MAX);
    let r2 = TaggedValue::new_raw_integer(U256::from(fake_rand()));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (_result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_lt_of);
}

#[test]
fn test_add_sets_eq_flag() {
    let bin_path = make_bin_path_asm("add_sets_overflow");
    let r1 = TaggedValue::new_raw_integer(U256::MAX);
    let r2 = TaggedValue::new_raw_integer(U256::one());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_eq);
    assert!(result.is_zero());
}

#[test]
fn test_add_sets_gt_flag_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("add_sets_gt_flag");
    let r1 = TaggedValue::new_raw_integer(U256::one());
    let r2 = TaggedValue::new_raw_integer(U256::one());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_gt);
    assert!(!final_vm_state.flag_eq);
    assert!(!final_vm_state.flag_lt_of);
    assert!(result == U256::from(2));
}

#[test]
fn test_add_does_not_modify_set_flags() {
    let bin_path = make_bin_path_asm("add_sub_do_not_modify_flags");
    // Trigger overflow on first add, so this sets the lt_of flag. Then a
    // non-overflowing add should leave the flag set.
    let r1 = TaggedValue::new_raw_integer(U256::MAX);
    let r2 = TaggedValue::new_raw_integer(fake_rand().into());
    let r3 = TaggedValue::new_raw_integer(U256::from(1_usize));
    let r4 = TaggedValue::new_raw_integer(U256::from(1_usize));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (_result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_lt_of);
    assert!(final_vm_state.flag_eq);
}

#[test]
fn test_sub_flags_r1_rs_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("sub_flags_r1_r2");
    let r1 = TaggedValue::new_raw_integer(U256::from(11));
    let r2 = TaggedValue::new_raw_integer(U256::from(300));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (_result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_lt_of);
    assert!(!final_vm_state.flag_gt);
    assert!(!final_vm_state.flag_eq);
}

#[test]
fn test_sub_sets_eq_flag_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("sub_flags_r1_r2");
    let r1 = TaggedValue::new_raw_integer(U256::from(fake_rand()));
    let r2 = r1;
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (_result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_eq);
    assert!(!final_vm_state.flag_lt_of);
    assert!(!final_vm_state.flag_gt);
}

#[test]
fn test_sub_sets_gt_flag_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("sub_flags_r1_r2");
    let r1 = TaggedValue::new_raw_integer(U256::from(250));
    let r2 = TaggedValue::new_raw_integer(U256::from(1));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (_result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_gt);
    assert!(!final_vm_state.flag_eq);
    assert!(!final_vm_state.flag_lt_of);
}
#[test]
fn test_sub_and_add() {
    let bin_path = make_bin_path_asm("sub_and_add");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("1").unwrap());
}

#[test]
fn test_mul_asm() {
    let bin_path = make_bin_path_asm("mul");
    let (_, vm) = run_program(&bin_path);
    let low = vm.get_register(3).value;
    let high = vm.get_register(4).value;

    assert_eq!(low, U256::from_dec_str("6").unwrap());
    assert_eq!(high, U256::zero());
}

#[test]
fn test_mul_big_asm() {
    let bin_path = make_bin_path_asm("mul_big");
    let r1 = TaggedValue::new_raw_integer(U256::MAX);
    let r2 = TaggedValue::new_raw_integer(U256::from(2));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();

    let (_, vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);

    let low = vm.get_register(3).value;
    let high = vm.get_register(4).value;

    assert_eq!(low, U256::MAX - 1);
    assert_eq!(high, U256::from(1)); // multiply by 2 == shift left by 1
}

#[test]
fn test_mul_zero_asm() {
    let bin_path = make_bin_path_asm("mul_zero");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_mul_codepage() {
    let bin_path = make_bin_path_asm("mul_codepage");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("126").unwrap());
}

#[test]
fn test_mul_sets_overflow_flag() {
    let bin_path = make_bin_path_asm("mul_sets_overflow");
    let r1 = TaggedValue::new_raw_integer(U256::MAX);
    let r2 = TaggedValue::new_raw_integer(U256::MAX);
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;

    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (_, vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(vm.flag_lt_of);
}

#[test]
fn test_mul_stack() {
    let bin_path = make_bin_path_asm("mul_stack");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("6").unwrap());
}

#[test]
fn test_mul_conditional_gt_set() {
    let bin_path = make_bin_path_asm("mul_conditional_gt");

    let vm_with_custom_flags = VMStateBuilder::new().gt_flag(true).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("42").unwrap());
}

#[test]
fn test_mul_conditional_gt_not_set() {
    let bin_path = make_bin_path_asm("mul_conditional_gt");

    let vm_with_custom_flags = VMStateBuilder::new().build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_div_asm() {
    let bin_path = make_bin_path_asm("div");
    let (_, vm) = run_program(&bin_path);
    let quotient_result = vm.get_register(3).value;
    let remainder_result = vm.get_register(4).value;

    // 25 / 6 = 4 remainder 1
    assert_eq!(quotient_result, U256::from_dec_str("4").unwrap());
    assert_eq!(remainder_result, U256::from_dec_str("1").unwrap());
}

#[test]
#[should_panic]
fn test_div_zero_asm() {
    let bin_path = make_bin_path_asm("div_zero");
    run_program(&bin_path);
}

#[test]
fn test_div_set_eq_flag() {
    let bin_path = make_bin_path_asm("div_set_eq_flag");
    let (_, vm) = run_program(&bin_path);

    assert!(vm.flag_eq);
}

#[test]
fn test_div_set_gt_flag() {
    let bin_path = make_bin_path_asm("div_set_gt_flag");
    run_program(&bin_path);
    let (_, vm) = run_program(&bin_path);

    assert!(vm.flag_gt);
}

#[test]
fn test_div_codepage() {
    let bin_path = make_bin_path_asm("div_codepage");
    let (_, vm) = run_program(&bin_path);
    let quotient_result = vm.get_register(3).value;
    let remainder_result = vm.get_register(4).value;

    // 42 / 3 = 14 remainder 0
    assert_eq!(quotient_result, U256::from_dec_str("14").unwrap());
    assert_eq!(remainder_result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_div_stack() {
    let bin_path = make_bin_path_asm("div_stack");
    let (_, vm) = run_program(&bin_path);
    let quotient_result = vm.get_register(3).value;
    let remainder_result = vm.get_register(4).value;

    // 42 / 3 = 14 remainder 0
    assert_eq!(quotient_result, U256::from_dec_str("14").unwrap());
    assert_eq!(remainder_result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_div_conditional_gt_set() {
    let bin_path = make_bin_path_asm("div_conditional_gt");

    let vm_with_custom_flags = VMStateBuilder::new().gt_flag(true).build();
    let (_, vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let quotient_result = vm.get_register(3).value;
    let remainder_result = vm.get_register(4).value;

    assert_eq!(quotient_result, U256::from_dec_str("14").unwrap());
    assert_eq!(remainder_result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_div_conditional_gt_not_set() {
    let bin_path = make_bin_path_asm("div_conditional_gt");

    let vm_with_custom_flags = VMStateBuilder::new().build();
    let (_, vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let quotient_result = vm.get_register(3).value;
    let remainder_result = vm.get_register(4).value;

    // program sets registers 3 and 4 at the beginning, and only changes them if the conditional is met
    assert_eq!(quotient_result, U256::from_dec_str("1").unwrap());
    assert_eq!(remainder_result, U256::from_dec_str("1").unwrap());
}

#[test]
fn test_more_complex_program_with_conditionals() {
    let bin_path = make_bin_path_asm("add_and_sub_with_conditionals");
    let vm_with_custom_flags = VMStateBuilder::new()
        .eq_flag(true)
        .gt_flag(false)
        .lt_of_flag(false)
        .build();
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    dbg!(_final_vm_state);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}
#[test]
// This test should run out of gas before
// the program can save a number 3 into the storage.
fn test_runs_out_of_gas_and_stops() {
    let bin_path = make_bin_path_asm("add_with_costs");
    let vm = VMStateBuilder::new().gas_left(5510).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_uses_expected_gas() {
    let bin_path = make_bin_path_asm("add_with_costs");
    let vm = VMStateBuilder::new().gas_left(5600).build();
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
    assert_eq!(final_vm_state.gas_left(), 0_u32);
}

#[test]
fn test_ptr_add() {
    let bin_path = make_bin_path_asm("add_ptr");
    let ptr = FatPointer::default();
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.offset, 5);
}

#[test]
fn test_ptr_add_initial_offset() {
    let bin_path = make_bin_path_asm("add_ptr");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.offset, 15);
}

#[test]
#[should_panic = "Src1 too large for ptr_add"]
fn test_ptr_add_panics_if_diff_too_big() {
    let bin_path = make_bin_path_asm("add_ptr_r2_set");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::one() << 33);
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Offset overflow in ptr_add"]
fn test_ptr_add_panics_if_offset_overflows() {
    let bin_path = make_bin_path_asm("add_ptr_r2_set");
    let ptr = FatPointer {
        offset: (1 << 31) - 1,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer((U256::one() << 32) - 1);
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_add"]
fn test_ptr_add_panics_if_src0_not_a_pointer() {
    let bin_path = make_bin_path_asm("add_ptr");
    let r1 = TaggedValue::new_raw_integer(U256::from(5));
    let r2 = TaggedValue::new_raw_integer(U256::one());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_add"]
fn test_ptr_add_panics_if_src1_is_a_pointer() {
    let bin_path = make_bin_path_asm("add_ptr_r2_set");
    let ptr = FatPointer {
        offset: (1 << 31) - 1,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_ptr_sub() {
    let bin_path = make_bin_path_asm("sub_ptr");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.offset, 5);
}

#[test]
#[should_panic = "Src1 too large for ptr_sub"]
fn test_ptr_sub_panics_if_diff_too_big() {
    let bin_path = make_bin_path_asm("sub_ptr_r2_set");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::one() << 33);
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Offset overflow in ptr_sub"]
fn test_ptr_sub_panics_if_offset_overflows() {
    let bin_path = make_bin_path_asm("sub_ptr_r2_set");
    let ptr = FatPointer::default();
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::one());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_sub"]
fn test_ptr_sub_panics_if_src0_not_a_pointer() {
    let bin_path = make_bin_path_asm("sub_ptr");
    let r1 = TaggedValue::new_raw_integer(U256::from(5));
    let r2 = TaggedValue::new_raw_integer(U256::one());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_sub"]
fn test_ptr_sub_panics_if_src1_is_a_pointer() {
    let bin_path = make_bin_path_asm("sub_ptr_r2_set");
    let ptr = FatPointer {
        offset: (1 << 31) - 1,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_ptr_add_big_number() {
    let bin_path = make_bin_path_asm("add_ptr_r2_set");
    let ptr = FatPointer::default();
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::from_str_radix("0xFFFFFFFF", 16).unwrap());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.offset, 0xFFFFFFFF);
}

#[test]
#[should_panic = "Invalid operands for ptr_add"]
fn test_add_removes_tag_pointer() {
    let bin_path = make_bin_path_asm("add_remove_tag_pointer");
    let ptr = FatPointer::default();
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_ptr_shrink() {
    let bin_path = make_bin_path_asm("shrink_ptr");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 10,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.len, 5);
}

#[test]
#[should_panic = "Src1 too large for ptr_shrink"]
fn test_ptr_shrink_panics_if_diff_too_big() {
    let bin_path = make_bin_path_asm("shrink_ptr_r2_set");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 10,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::one() << 33);
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Len overflow in ptr_shrink"]
fn test_ptr_shrink_panics_if_offset_overflows() {
    let bin_path = make_bin_path_asm("shrink_ptr_r2_set");
    let ptr = FatPointer::default();
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::one());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_shrink"]
fn test_ptr_shrink_panics_if_src0_not_a_pointer() {
    let bin_path = make_bin_path_asm("shrink_ptr");
    let r1 = TaggedValue::new_raw_integer(U256::from(5));
    let r2 = TaggedValue::new_raw_integer(U256::one());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_shrink"]
fn test_ptr_shrink_panics_if_src1_is_a_pointer() {
    let bin_path = make_bin_path_asm("shrink_ptr_r2_set");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: (1 << 31) - 1,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_ptr_pack() {
    let bin_path = make_bin_path_asm("pack_ptr");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(
        U256::from_str_radix("0x100000000000000000000000000000000", 16).unwrap(),
    );
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(
        result,
        U256::from_str_radix("0x100000000000000000000000000000000", 16).unwrap()
    );
}

#[test]
fn test_ptr_pack_max_value() {
    let bin_path = make_bin_path_asm("pack_ptr");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(
        U256::from_str_radix(
            "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000000000000000000000000000",
            16,
        )
        .unwrap(),
    );
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(
        result,
        U256::from_str_radix(
            "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000000000000000000000000000",
            16
        )
        .unwrap()
    );
}

#[test]
fn test_ptr_pack_pointer_not_empty() {
    let bin_path = make_bin_path_asm("pack_ptr");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 10,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(
        U256::from_str_radix("0x100000000000000000000000000000000", 16).unwrap(),
    );
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.len, 10);
}

#[test]
#[should_panic = "Src1 low 128 bits not 0"]
fn test_ptr_pack_diff_incorrect_value() {
    let bin_path = make_bin_path_asm("pack_ptr");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 10,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(
        U256::from_str_radix("0x100000000000000000000000000100000", 16).unwrap(),
    );
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_pack"]
fn test_ptr_pack_panics_if_src0_not_a_pointer() {
    let bin_path = make_bin_path_asm("pack_ptr");
    let r1 = TaggedValue::new_raw_integer(U256::from(5));
    let r2 = TaggedValue::new_raw_integer(
        U256::from_str_radix("0x100000000000000000000000000100000", 16).unwrap(),
    );
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for ptr_pack"]
fn test_ptr_pack_panics_if_src1_is_a_pointer() {
    let bin_path = make_bin_path_asm("pack_ptr");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: (1 << 31) - 1,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_ptr_add_in_stack() {
    let bin_path = make_bin_path_asm("add_ptr_stack");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.offset, 15);
}

#[test]
fn test_ptr_sub_in_stack() {
    let bin_path = make_bin_path_asm("sub_ptr_stack");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.offset, 5);
}

#[test]
fn test_ptr_shrink_in_stack() {
    let bin_path = make_bin_path_asm("shrink_ptr_stack");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 10,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_ptr = FatPointer::decode(result);
    assert_eq!(new_ptr.len, 5);
}

#[test]
fn test_ptr_pack_in_stack() {
    let bin_path = make_bin_path_asm("pack_ptr_stack");
    let ptr = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(
        U256::from_str_radix("0x100000000000000000000000000000000", 16).unwrap(),
    );
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(
        result,
        U256::from_str_radix("0x100000000000000000000000000000000", 16).unwrap()
    );
}

#[test]
fn test_heap() {
    let bin_path = make_bin_path_asm("heap");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(10));
}

#[test]
fn test_heap_offset_not_0() {
    let bin_path = make_bin_path_asm("heap");
    let r1 = TaggedValue::new_raw_integer(U256::from(5));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(10));
}

#[test]
fn test_heap_two_addresses_replace() {
    let bin_path = make_bin_path_asm("heap_two_addresses");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let r3 = TaggedValue::new_raw_integer(U256::from(0));
    let r4 = TaggedValue::new_raw_integer(U256::from(15));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(15));
}

#[test]
fn test_heap_two_addresses_overlap() {
    let bin_path = make_bin_path_asm("heap_two_addresses");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let r3 = TaggedValue::new_raw_integer(U256::from(10));
    let r4 = TaggedValue::new_raw_integer(U256::from(15));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(15));
}

#[test]
fn test_heap_two_addresses_recover_first() {
    let bin_path = make_bin_path_asm("heap_two_addresses_first");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let r3 = TaggedValue::new_raw_integer(U256::from(10));
    let r4 = TaggedValue::new_raw_integer(U256::from(15));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
}

#[test]
#[should_panic = "Address too large for heap_write"]
fn test_heap_offset_too_big() {
    let bin_path = make_bin_path_asm("heap");
    let r1 = TaggedValue::new_raw_integer(U256::from(0xFFFFFFE0_u32));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for heap_write"]
fn test_heap_invalid_operands() {
    let bin_path = make_bin_path_asm("heap");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_heap_only_read() {
    let bin_path = make_bin_path_asm("heap_only_read");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
}

#[test]
fn test_heap_only_read_offset() {
    let bin_path = make_bin_path_asm("heap_only_read");
    let r1 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
}

#[test]
#[should_panic = "Address too large for heap_read"]
fn test_heap_only_read_offset_too_large() {
    let bin_path = make_bin_path_asm("heap_only_read");
    let r1 = TaggedValue::new_raw_integer(U256::from(0xFFFFFFE0_u32));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for heap_read"]
fn test_heap_only_read_invalid_operand() {
    let bin_path = make_bin_path_asm("heap_only_read");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_heap_store_inc() {
    let bin_path = make_bin_path_asm("heap_store_inc");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, new_vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(10));
    assert_eq!(new_vm.registers[2].value, U256::from(32));
}

#[test]
fn test_heap_load_inc() {
    let bin_path = make_bin_path_asm("heap_load_inc");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, new_vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
    assert_eq!(new_vm.registers[3].value, U256::from(32));
}

#[test]
fn test_fat_pointer_read() {
    let bin_path = make_bin_path_asm("fat_pointer_read");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let pointer = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 32,
    };
    let r3 = TaggedValue::new_pointer(pointer.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(10));
}

#[test]
fn test_fat_pointer_read_len_zero() {
    let bin_path = make_bin_path_asm("fat_pointer_read");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let pointer = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 0,
    };
    let r3 = TaggedValue::new_pointer(pointer.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
}

#[test]
fn test_fat_pointer_read_start_and_offset() {
    let bin_path = make_bin_path_asm("fat_pointer_read");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(
        U256::from_str_radix(
            "0x123456789ABCDEF0123400000000000000000000000000000000000000000001",
            16,
        )
        .unwrap(),
    );
    let pointer = FatPointer {
        offset: 3,
        page: 0,
        start: 2,
        len: 10,
    };
    let r3 = TaggedValue::new_pointer(pointer.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(
        result,
        U256::from_str_radix(
            "0xBCDEF01234000000000000000000000000000000000000000000000000000000",
            16
        )
        .unwrap()
    );
}

#[test]
fn test_fat_pointer_read_inc() {
    let bin_path = make_bin_path_asm("fat_pointer_read_inc");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let pointer = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 64,
    };
    let r3 = TaggedValue::new_pointer(pointer.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    let new_pointer = FatPointer::decode(result);
    assert_eq!(new_pointer.offset, 32);
}

#[test]
#[should_panic = "Invalid operands for fat_pointer_read"]
fn test_fat_pointer_read_not_a_pointer() {
    let bin_path = make_bin_path_asm("fat_pointer_read");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let pointer = FatPointer {
        offset: 0,
        page: 0,
        start: 0,
        len: 32,
    };
    let r3 = TaggedValue::new_raw_integer(pointer.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_heap_aux() {
    let bin_path = make_bin_path_asm("heap_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(10));
}

#[test]
fn test_heap_offset_not_0_aux() {
    let bin_path = make_bin_path_asm("heap_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(5));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(10));
}

#[test]
fn test_heap_two_addresses_replace_aux() {
    let bin_path = make_bin_path_asm("heap_two_addresses_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let r3 = TaggedValue::new_raw_integer(U256::from(0));
    let r4 = TaggedValue::new_raw_integer(U256::from(15));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(15));
}

#[test]
fn test_heap_two_addresses_overlap_aux() {
    let bin_path = make_bin_path_asm("heap_two_addresses_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let r3 = TaggedValue::new_raw_integer(U256::from(10));
    let r4 = TaggedValue::new_raw_integer(U256::from(15));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(15));
}

#[test]
fn test_heap_two_addresses_recover_first_aux() {
    let bin_path = make_bin_path_asm("heap_two_addresses_first_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let r3 = TaggedValue::new_raw_integer(U256::from(10));
    let r4 = TaggedValue::new_raw_integer(U256::from(15));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
}

#[test]
#[should_panic = "Address too large for heap_write"]
fn test_heap_offset_too_big_aux() {
    let bin_path = make_bin_path_asm("heap_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0xFFFFFFE0_u32));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for heap_write"]
fn test_heap_invalid_operands_aux() {
    let bin_path = make_bin_path_asm("heap_aux");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_heap_only_read_aux() {
    let bin_path = make_bin_path_asm("heap_only_read_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
}

#[test]
fn test_heap_only_read_offset_aux() {
    let bin_path = make_bin_path_asm("heap_only_read_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, _) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
}

#[test]
#[should_panic = "Address too large for heap_read"]
fn test_heap_only_read_offset_too_large_aux() {
    let bin_path = make_bin_path_asm("heap_only_read_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0xFFFFFFE0_u32));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
#[should_panic = "Invalid operands for heap_read"]
fn test_heap_only_read_invalid_operand_aux() {
    let bin_path = make_bin_path_asm("heap_only_read_aux");
    let ptr = FatPointer {
        offset: 10,
        page: 0,
        start: 0,
        len: 0,
    };
    let r1 = TaggedValue::new_pointer(ptr.encode());
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    run_program_with_custom_state(&bin_path, vm_with_custom_flags);
}

#[test]
fn test_heap_store_inc_aux() {
    let bin_path = make_bin_path_asm("heap_store_inc_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, new_vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(10));
    assert_eq!(new_vm.registers[2].value, U256::from(32));
}

#[test]
fn test_heap_load_inc_aux() {
    let bin_path = make_bin_path_asm("heap_load_inc_aux");
    let r1 = TaggedValue::new_raw_integer(U256::from(0));
    let r2 = TaggedValue::new_raw_integer(U256::from(10));
    let mut registers: [TaggedValue; 15] = [TaggedValue::default(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMStateBuilder::new().with_registers(registers).build();
    let (result, new_vm) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from(0));
    assert_eq!(new_vm.registers[3].value, U256::from(32));
}

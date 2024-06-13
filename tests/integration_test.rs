use era_vm::{run_program, run_program_with_custom_state, state::VMState};
use u256::U256;
const ARTIFACTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/program_artifacts");

fn make_bin_path_yul(file_name: &str) -> String {
    format!(
        "{}/{}.artifacts.yul/{}.yul.zbin",
        ARTIFACTS_PATH, file_name, file_name
    )
}

fn make_bin_path_asm(file_name: &str) -> String {
    format!(
        "{}/{}.artifacts.zasm/{}.zasm.zbin",
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
    let vm_with_custom_flags = VMState::new_with_flag_state(false, true, false);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

#[test]
fn test_add_does_run_if_lt_is_set() {
    let bin_path = make_bin_path_asm("add_conditional_lt");
    let vm_with_custom_flags = VMState::new_with_flag_state(true, false, true);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

#[test]
fn test_add_does_not_run_if_lt_is_not_set() {
    let bin_path = make_bin_path_asm("add_conditional_not_lt");
    let vm_with_custom_flags = VMState::new_with_flag_state(true, false, true);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

#[test]
fn test_add_does_run_if_gt_is_set() {
    let bin_path = make_bin_path_asm("add_conditional_gt");
    let vm_with_custom_flags = VMState::new_with_flag_state(true, false, true);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("20").unwrap());
}

#[test]
fn test_add_does_not_run_if_gt_is_not_set() {
    let bin_path = make_bin_path_asm("add_conditional_not_gt");
    let vm_with_custom_flags = VMState::new_with_flag_state(false, false, false);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
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
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("6").unwrap());
}

#[test]
fn test_mul_zero_asm() {
    let bin_path = make_bin_path_asm("mul_zero");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_div_asm() {
    // for this test we need to run two identical programs
    // that only differ in their return values

    let first_bin_path = make_bin_path_asm("div_quotient");
    let (quotient_result, _) = run_program(&first_bin_path);

    let second_bin_path = make_bin_path_asm("div_remainder");
    let (remainder_result, _) = run_program(&second_bin_path);

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
fn test_more_complex_program_with_conditionals() {
    let bin_path = make_bin_path_asm("add_and_sub_with_conditionals");
    let vm_with_custom_flags = VMState::new_with_flag_state(false, true, false);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

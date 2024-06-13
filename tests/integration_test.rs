use era_vm::{run_program, run_program_with_custom_state, state::VMState};
use std::{
    ops::Sub,
    time::{SystemTime, UNIX_EPOCH},
};
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
fn test_sub_and_add() {
    let bin_path = make_bin_path_asm("sub_and_add");
    let (result, _) = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("1").unwrap());
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
fn test_more_complex_program_with_conditionals() {
    let bin_path = make_bin_path_asm("add_and_sub_with_conditionals");
    let vm_with_custom_flags = VMState::new_with_flag_state(false, true, false);
    let (result, _final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert_eq!(result, U256::from_dec_str("10").unwrap());
}

#[test]
fn test_add_sets_overflow_flag() {
    let bin_path = make_bin_path_asm("add_sets_overflow");
    let r1 = U256::MAX;
    let r2 = U256::from(fake_rand());
    let mut registers: [U256; 15] = [U256::zero(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMState::new_with_registers(registers);
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_lt_of);
}

#[test]
fn test_add_sets_eq_flag() {
    let bin_path = make_bin_path_asm("add_sets_overflow");
    let r1 = U256::MAX;
    let r2 = U256::from(1);
    let mut registers: [U256; 15] = [U256::zero(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMState::new_with_registers(registers);
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_eq);
    assert!(result.is_zero());
}

#[test]
fn test_add_sets_gt_flag_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("add_sets_gt_flag");
    let r1 = U256::from(1);
    let r2 = U256::from(1);
    let mut registers: [U256; 15] = [U256::zero(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMState::new_with_registers(registers);
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_gt);
    assert!(!final_vm_state.flag_eq);
    assert!(!final_vm_state.flag_lt_of);
    assert!(result == U256::from(2));
}

#[test]
fn test_sub_flags_r1_rs_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("sub_flags_r1_r2");
    let r1 = U256::from(11);
    let r2 = U256::from(300);
    let mut registers: [U256; 15] = [U256::zero(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMState::new_with_registers(registers);
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_lt_of);
    assert!(!final_vm_state.flag_gt);
    assert!(!final_vm_state.flag_eq);
}

#[test]
fn test_sub_sets_eq_flag_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("sub_flags_r1_r2");
    let r1 = U256::from(fake_rand());
    let r2 = r1.clone();
    let mut registers: [U256; 15] = [U256::zero(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMState::new_with_registers(registers);
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_eq);
    assert!(!final_vm_state.flag_lt_of);
    assert!(!final_vm_state.flag_gt);
}

#[test]
fn test_sub_sets_gt_flag_keeps_other_flags_clear() {
    let bin_path = make_bin_path_asm("sub_flags_r1_r2");
    let r1 = U256::from(250);
    let r2 = U256::from(1);
    let mut registers: [U256; 15] = [U256::zero(); 15];
    registers[0] = r1;
    registers[1] = r2;
    let vm_with_custom_flags = VMState::new_with_registers(registers);
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_gt);
    assert!(!final_vm_state.flag_eq);
    assert!(!final_vm_state.flag_lt_of);
}

#[test]
fn test_add_does_not_modify_set_flags() {
    let bin_path = make_bin_path_asm("add_sub_do_not_modify_flags");
    // Trigger overflow on first add, so this sets the lt_of flag. Then a
    // non-overflowing add should leave the flag set.
    let r1 = U256::MAX;
    let r2 = fake_rand().into();
    let r3 = U256::from(1_usize);
    let r4 = U256::from(1_usize);
    let mut registers: [U256; 15] = [U256::zero(); 15];
    registers[0] = r1;
    registers[1] = r2;
    registers[2] = r3;
    registers[3] = r4;
    let vm_with_custom_flags = VMState::new_with_registers(registers);
    let (result, final_vm_state) = run_program_with_custom_state(&bin_path, vm_with_custom_flags);
    assert!(final_vm_state.flag_lt_of);
    assert!(final_vm_state.flag_eq);
}

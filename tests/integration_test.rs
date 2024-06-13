use era_vm::run_program;
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
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
fn test_add_asm() {
    let bin_path = make_bin_path_asm("add");
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
fn test_sub_asm_simple() {
    let bin_path = make_bin_path_asm("sub_simple");
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

#[test]
fn test_sub_asm() {
    let bin_path = make_bin_path_asm("sub_should_be_zero");
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_sub_and_add() {
    let bin_path = make_bin_path_asm("sub_and_add");
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("1").unwrap());
}

#[test]
fn test_mul_asm() {
    let bin_path = make_bin_path_asm("mul");
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("6").unwrap());
}

#[test]
fn test_mul_zero_asm() {
    let bin_path = make_bin_path_asm("mul_zero");
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("0").unwrap());
}

#[test]
fn test_div_asm() {
    // for this test we need to run two identical programs
    // that only differ in their return values

    let first_bin_path = make_bin_path_asm("div_quotient");
    let quotient_result = run_program(&first_bin_path);

    let second_bin_path = make_bin_path_asm("div_remainder");
    let remainder_result = run_program(&second_bin_path);

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

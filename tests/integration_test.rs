use era_vm::run_program;
use u256::U256;

#[test]
fn test_add() {
    let bin_path = "./program_artifacts/add.artifacts/add.yul.zbin";
    let result = run_program(&bin_path);
    assert_eq!(result, U256::from_dec_str("3").unwrap());
}

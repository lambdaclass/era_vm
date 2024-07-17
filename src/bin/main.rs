use std::env;
use std::process::exit;

use era_vm::run_program;
use era_vm::state::VMStateBuilder;

fn main() {
    let args: Vec<String> = env::args().collect();
    let bin_path = match args.get(1) {
        Some(path) => path,
        None => {
            println!("Pass a program to run");
            exit(1);
        }
    };

    let vm = VMStateBuilder::default().build();
    let output = run_program(bin_path, vm, &mut []);
    println!("RESULT: {:?}", output.storage_zero);
}

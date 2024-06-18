use std::env;
use std::process::exit;

use era_vm::run_program_in_memory;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Pass a program to run");
        exit(1);
    }

    let result = run_program_in_memory(args.get(1).unwrap());
    println!("RESULT: {:?}", result);
}

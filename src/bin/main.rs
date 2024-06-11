use std::env;
use std::process::exit;

use era_vm::run_program;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Pass a program to run");
        exit(1);
    }

    let result = run_program(args.get(1).unwrap());
    println!("RESULT: {:?}", result);
}

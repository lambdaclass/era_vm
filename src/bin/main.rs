use std::env;
use std::process::exit;

use era_vm::run_program;
use era_vm::state::VMStateBuilder;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Pass a program to run");
        exit(1);
    }

    let vm = VMStateBuilder::default().build();
    let result = run_program(args.get(1).unwrap(), vm, &mut [], None, None);
    println!("RESULT: {:?}", result);
}

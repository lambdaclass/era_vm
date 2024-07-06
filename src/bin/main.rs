use std::env;
use std::process::exit;

use era_vm::run_program;
use era_vm::state::VMStateBuilder;
use era_vm::tracers::print_tracer::PrintTracer;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Pass a program to run");
        exit(1);
    }

    let vm = VMStateBuilder::default().build();
    let mut tracer = PrintTracer {};
    let result = run_program(args.get(1).unwrap(), vm, &mut [Box::new(&mut tracer)]);
    println!("RESULT: {:?}", result);
}

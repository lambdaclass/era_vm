use std::env;
use std::process::exit;
use std::str::FromStr;

use era_vm::run_program;
use era_vm::state::VMStateBuilder;
use era_vm::store::InMemory;
use era_vm::tracers::print_tracer::PrintTracer;
use u256::H160;

fn main() {
    // let args: Vec<String> = env::args().collect();
    // if args.len() <= 1 {
    //     println!("Pass a program to run");
    //     exit(1);
    // }

    // let vm = VMStateBuilder::default().build();
    // let mut tracer = PrintTracer {};
    // let mut storage = InMemory::new_empty();
    // let result = run_program(
    //     args.get(1).unwrap(),
    //     vm,
    //     &mut storage,
    //     &mut [Box::new(&mut tracer)],
    //     // Mocked address
    //     &H160::from_str("0x1000000000100000000010000000001000000000").unwrap(),
    // );
    // println!("RESULT: {:?}", result);
}

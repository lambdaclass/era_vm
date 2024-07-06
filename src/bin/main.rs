use std::env;
use std::process::exit;

use era_vm::run_program;
use era_vm::state::VMStateBuilder;
use era_vm::store::InMemory;
use era_vm::tracers::print_tracer::PrintTracer;
use era_vm::world_state::WorldState;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Pass a program to run");
        exit(1);
    }

    let vm = VMStateBuilder::default().build();
    let mut tracer = PrintTracer {};
    let world_state = WorldState::new(Box::new(InMemory::new_empty()));
    let result = run_program(
        args.get(1).unwrap(),
        vm,
        world_state,
        &mut [Box::new(&mut tracer)],
    );
    println!("RESULT: {:?}", result);
}

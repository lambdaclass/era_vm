mod opcode;
mod state;
mod value;
mod op_handlers;

use std::env;
use std::process::exit;

use op_handlers::add::_add;
use opcode::Opcode;
use state::VMState;
use u256::U256;
use zkevm_opcode_defs::definitions::synthesize_opcode_decoding_tables;
use zkevm_opcode_defs::ISAVersion;
use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::LogOpcode;

fn main() {
    let opcode_table = synthesize_opcode_decoding_tables(11, ISAVersion(2));

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Pass a program to run");
        exit(1);
    }

    let program_path = args.get(1).unwrap();
    // let program = std::fs::read("./program_artifacts/add.yul.zbin").unwrap();
    let program = std::fs::read(program_path).unwrap();
    let encoded = String::from_utf8(program.to_vec()).unwrap();
    let bin = hex::decode(encoded[2..].to_owned()).unwrap();

    let mut program_code = vec![];
    for raw_opcode_slice in bin.chunks(8) {
        let mut raw_opcode_bytes: [u8; 8] = [0; 8];
        for i in 0..8 {
            raw_opcode_bytes[i] = raw_opcode_slice[i];
        }
        
        let raw_opcode_u64 = u64::from_be_bytes(raw_opcode_bytes);
        let opcode = Opcode::from_raw_opcode(raw_opcode_u64, &opcode_table);
        program_code.push(opcode);
    }

    let mut vm = VMState::new(program_code);

    loop {
        let opcode = vm.current_frame.code_page[vm.current_frame.pc as usize].clone();
        match opcode.variant {
            Variant::Invalid(_) => todo!(),
            Variant::Nop(_) => todo!(),
            Variant::Add(_) => {
                _add(&mut vm, opcode);
            },
            Variant::Sub(_) => todo!(),
            Variant::Mul(_) => todo!(),
            Variant::Div(_) => todo!(),
            Variant::Jump(_) => todo!(),
            Variant::Context(_) => todo!(),
            Variant::Shift(_) => todo!(),
            Variant::Binop(_) => todo!(),
            Variant::Ptr(_) => todo!(),
            Variant::NearCall(_) => todo!(),
            Variant::Log(log_variant) => {
                match log_variant {
                    LogOpcode::StorageRead => todo!(),
                    LogOpcode::StorageWrite => {
                        let src0 = vm.get_register(opcode.src0_index);
                        let src1 = vm.get_register(opcode.src1_index);
                        vm.current_frame.storage.insert(src0, src1);
                    },
                    LogOpcode::ToL1Message => todo!(),
                    LogOpcode::Event => todo!(),
                    LogOpcode::PrecompileCall => todo!(),
                    LogOpcode::Decommit => todo!(),
                    LogOpcode::TransientStorageRead => todo!(),
                    LogOpcode::TransientStorageWrite => todo!(),
                }
            },
            Variant::FarCall(_) => todo!(),
            Variant::Ret(_) => {
                // TODO: This is not how return works. Fix when we have calls between contracts
                // hooked up.
                break;
            },
            Variant::UMA(_) => todo!(),
        }

        vm.current_frame.pc += 1;
    }

    println!("RESULT: {:?}", vm.current_frame.storage.get(&U256::zero()).unwrap());
}

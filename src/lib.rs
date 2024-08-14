mod address_operands;
pub mod call_frame;
mod debug;
mod eravm_error;
pub mod heaps;
mod op_handlers;
mod opcode;
pub mod output;
mod precompiles;
mod ptr_operator;
pub mod state;
pub mod store;
pub mod tracers;
pub mod utils;
pub mod value;
pub mod vm;
pub use opcode::Opcode;
pub use state::VMState;
pub use vm::EraVM;
use zkevm_opcode_defs::Opcode as Variant;

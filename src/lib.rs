mod address_operands;
pub mod call_frame;
mod eravm_error;
pub mod execution;
pub mod heaps;
mod op_handlers;
mod opcode;
pub mod output;
mod precompiles;
mod ptr_operator;
pub mod store;
pub mod tracers;
pub mod utils;
pub mod value;
pub mod vm;
pub use execution::Execution;
pub use opcode::Opcode;
pub use store::Storage;
pub use vm::EraVM;
pub mod rollbacks;
pub mod state;
use zkevm_opcode_defs::Opcode as Variant;

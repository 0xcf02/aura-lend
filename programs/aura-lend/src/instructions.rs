pub mod market_instructions;
pub mod lending_instructions;
pub mod borrowing_instructions;
pub mod liquidation_instructions;
pub mod oracle_instructions;
pub mod multisig_instructions;
pub mod timelock_instructions;
pub mod governance_instructions;

// Re-export all instructions and their context structs
pub use market_instructions::*;
pub use lending_instructions::*;
pub use borrowing_instructions::*;
pub use liquidation_instructions::*;
pub use oracle_instructions::*;
pub use multisig_instructions::*;
pub use timelock_instructions::*;
pub use governance_instructions::*;
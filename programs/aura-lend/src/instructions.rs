pub mod market_instructions;
pub mod lending_instructions;
pub mod borrowing_instructions;
pub mod liquidation_instructions;
pub mod oracle_instructions;

// Re-export all instructions and their context structs
pub use market_instructions::*;
pub use lending_instructions::*;
pub use borrowing_instructions::*;
pub use liquidation_instructions::*;
pub use oracle_instructions::*;
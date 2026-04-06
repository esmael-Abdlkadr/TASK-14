pub mod assignment;
pub mod consistency;
#[cfg(test)]
mod consistency_tests;
#[cfg(test)]
mod coi_tests;

pub use assignment::*;
pub use consistency::*;

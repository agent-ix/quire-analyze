//! SMT-backed consistency and implication analysis for versioned requirement contracts.

mod analysis;
mod datatype_replay;
mod hybrid_synthesis;
mod package_runner;
mod package_translation;
mod producer_evidence;
mod report;
mod smt;
mod solver;

pub use analysis::*;
pub use datatype_replay::WitnessValue;
pub use hybrid_synthesis::*;
pub use package_runner::*;
pub use package_translation::*;
pub use producer_evidence::*;
pub use report::*;
pub use smt::*;
pub use solver::*;

/// Placeholder entry point.
pub fn hello() -> &'static str {
    "hello from quire_analyze"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert!(hello().contains("quire_analyze"));
    }
}

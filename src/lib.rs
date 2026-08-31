//! SMT-backed consistency and implication analysis for versioned requirement contracts.

mod smt;
mod solver;

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

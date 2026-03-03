use kernel_contracts::contract::Contract;
use kernel_contracts::compiler::compile_contract;

/// The GoldMaster suite S = {Q_i}.
///
/// A fixed set of computational contracts that stress:
/// 1. Determinism (same input → same output, same trace)
/// 2. Replay correctness (receipts verify)
/// 3. UNIQUE/UNSAT/Ω coverage (all three statuses exercised)
/// 4. Self-recognition (traces match predictions)
///
/// This suite defines the kernel's identity under Π.
/// Two builds are "the same" iff indistinguishable on S.
pub struct GoldMasterSuite {
    pub contracts: Vec<Contract>,
}

impl GoldMasterSuite {
    /// Build the canonical GoldMaster suite v1.
    ///
    /// 10 contracts covering:
    /// - Simple SAT (UNIQUE)
    /// - UNSAT (contradiction)
    /// - Multiple solutions (tiebreak → UNIQUE)
    /// - Arithmetic (UNIQUE)
    /// - Arithmetic (UNSAT)
    /// - Table lookup (UNIQUE)
    /// - Table lookup (UNSAT)
    /// - Large boolean formula (determinism stress)
    /// - Trivial tautology (immediate UNIQUE)
    /// - Edge case: single variable
    pub fn v1() -> Self {
        let specs = vec![
            // Q0: Simple 2-var SAT, one clause. Multiple solutions → tiebreak.
            r#"{"type":"bool_cnf","description":"Q0: x1 OR x2","num_vars":2,"clauses":[[1,2]]}"#,

            // Q1: UNSAT — x AND NOT x.
            r#"{"type":"bool_cnf","description":"Q1: x AND NOT x (UNSAT)","num_vars":1,"clauses":[[1],[-1]]}"#,

            // Q2: Unique SAT — (x1) AND (x2) AND (NOT x1 OR x2).
            r#"{"type":"bool_cnf","description":"Q2: forced x1=T x2=T","num_vars":2,"clauses":[[1],[2],[-1,2]]}"#,

            // Q3: 3-var with multiple clauses.
            r#"{"type":"bool_cnf","description":"Q3: 3-var mixed","num_vars":3,"clauses":[[1,2,3],[-1,2],[-2,3],[-3,1]]}"#,

            // Q4: Arithmetic — find x: 2x + 3 = 7 → x = 2 (UNIQUE).
            r#"{"type":"arith_find","description":"Q4: 2x+3=7","coefficients":[3,2],"target":7,"lo":-10,"hi":10}"#,

            // Q5: Arithmetic — find x: x^2 = -1 (UNSAT over integers).
            r#"{"type":"arith_find","description":"Q5: x^2=-1 (UNSAT)","coefficients":[0,0,1],"target":-1,"lo":-10,"hi":10}"#,

            // Q6: Table lookup — one SAT entry.
            r#"{"type":"table","description":"Q6: table unique","entries":[{"key":"alpha","value":"UNSAT"},{"key":"beta","value":"SAT"},{"key":"gamma","value":"UNSAT"}]}"#,

            // Q7: Table lookup — no SAT entries.
            r#"{"type":"table","description":"Q7: table UNSAT","entries":[{"key":"a","value":"UNSAT"},{"key":"b","value":"UNSAT"}]}"#,

            // Q8: 4-var determinism stress.
            r#"{"type":"bool_cnf","description":"Q8: 4-var stress","num_vars":4,"clauses":[[1,2],[3,4],[-1,-3],[2,-4],[-2,3,4],[1,-2,-3,-4]]}"#,

            // Q9: Single variable, single positive clause → x=true is forced.
            r#"{"type":"bool_cnf","description":"Q9: single var forced true","num_vars":1,"clauses":[[1]]}"#,
        ];

        let contracts: Vec<Contract> = specs.iter()
            .map(|s| compile_contract(s).expect("GoldMaster contract must compile"))
            .collect();

        GoldMasterSuite { contracts }
    }

    /// Number of contracts in the suite.
    pub fn len(&self) -> usize {
        self.contracts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_v1_compiles() {
        let suite = GoldMasterSuite::v1();
        assert_eq!(suite.len(), 10);
        for (i, c) in suite.contracts.iter().enumerate() {
            assert!(!c.description.is_empty(), "Contract Q{} has empty description", i);
        }
    }
}

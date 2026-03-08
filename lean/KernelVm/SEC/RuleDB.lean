/-!
# SEC Rule Database — Lean4 Registry

The rule database is a persistent, ordered collection of proven rules.
Each rule has a soundness theorem that type-checks in Lean4.
The database is Merkle-committed in the Rust kernel.

This file provides the Lean-side registry structure.
-/

import KernelVm.SEC.RuleSyn

namespace KernelVm.SEC

/-- A proven rule: a schema together with its soundness proof. -/
structure ProvenRule where
  schema : RuleSyn
  soundness : Sound schema
  theoremName : String
  ruleHash : UInt64

/-- The rule database: a list of proven rules in discovery order. -/
structure RuleDB where
  rules : List ProvenRule

/-- Empty rule database. -/
def RuleDB.empty : RuleDB := ⟨[]⟩

/-- Add a proven rule to the database. -/
def RuleDB.addRule (db : RuleDB) (rule : ProvenRule) : RuleDB :=
  ⟨db.rules ++ [rule]⟩

/-- Number of rules in the database. -/
def RuleDB.size (db : RuleDB) : Nat := db.rules.length

end KernelVm.SEC

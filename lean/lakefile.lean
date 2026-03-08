import Lake
open Lake DSL

package kernelVm where
  leanOptions := #[
    ⟨`autoImplicit, false⟩
  ]

require mathlib from git
  "https://github.com/leanprover-community/mathlib4" @ "v4.16.0"

@[default_target]
lean_lib KernelVm where
  srcDir := "."

@[default_target]
lean_lib OpenProblems where
  srcDir := "."

@[default_target]
lean_lib Frontier where
  srcDir := "."

@[default_target]
lean_lib ProofEnum where
  srcDir := "."

@[default_target]
lean_lib Generated where
  srcDir := "."

@[default_target]
lean_lib Universe where
  srcDir := "."

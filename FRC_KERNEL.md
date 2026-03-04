# What FRC Changes for a Self-Computing, Fixed-Point Kernel

FRC (Finite Reduction Certificate) changes the kernel's outer loop, not the per-tick controller:
- Per tick you already want a fixed point:
  a_t = a*(X_t)  (one Π-canonical joint action, no "robot decisions")
- Across the whole run, FRC forces completion: the kernel may not say "couldn't within budget." It must either:
  - produce a finite computation that halts with a win witness, or
  - produce a finite computation that halts with a refutation (over a declared finite family), or
  - return INVALID with the minimal missing-lemma witness that blocks the reduction.

So FRC turns "self-aware fixed point each tick" into a self-contained compiler-and-checker: the kernel models the instance, derives a finite reduction bound B*, runs to completion, and emits UNIQUE/UNSAT/INVALID—no Ω.

---

## 1) The full math object

### 1.1 Pinned universe

X_{t+1} = Exec_I(X_t, a_t),  t = 0,...,4999,  a_t ∈ {FW,CR,CCR,W}^{10000}.

Pool law:
|P_t| = 15000,  P_{t+1} = P_t \ {completed} ∪ {next templates}.

### 1.2 The self-aware per-tick fixed point

Define the admissible action cone:
A(X) = {a : Exec_I(X,a) does not rollback/timeout}.

Define the Π-quotient state (the only oracle-distinguished state you control):
Π(X) = (H_z, Q_j, P_t, C_t)
(holes per zone H_z, junction pressures Q_j, pool state P_t, completions C_t).

Let I* ⊆ Π(X) be the maximal controllable invariant set in quotient space:
I* = gfp(S ↦ {Π(X) : ∃a ∈ A(X), Π(Exec(X,a)) ∈ S}).

Then the self-aware tick law is:

**a*(X) = Π-min{a ∈ A(X) : Π(Exec(X,a)) ∈ I*}.**

This is the "conscious fixed point each tick": choose the Π-canonical action that stays inside the invariant region.

### 1.3 FRC / A1′ completion (outer fixed point)

For a win claim S ("there exists a controller that beats target"), A1′ requires an FRC:

FRC(S) = (C, B*, π, ProofEq, ProofTotal)

such that:
- C is a self-delimiting finite computation,
- B* is derived internally,
- π = run(C) halts within B*,
- ProofEq: S ⟺ (π returns 1),
- ProofTotal: halting within B*.

Key point: LoRR evaluation already has B* = 5000 ticks for any fixed policy; the only nontrivial part is making the search for a winning policy finite.

So we define the admissible "solve LoRR" contract as:

S_win(P) := ∃P ∈ P : Score_I(P) > 154,834

for a finite policy family P. Then:

B*(S_win(P)) = |P| · 5000.

No Ω.

---

## 2) What the kernel must do "by itself"

It must do three things, deterministically:
1. **Model the instance**: compile K_terrain and K_pool from pinned files.
2. **Derive a finite policy family P** (the only source of "options," but fully Π-ordered and internal).
3. **Complete**: enumerate P, run each P for 5000 ticks, stop at first win, else refute.

The "self fixed point each tick" is inside each P. FRC makes the outer loop complete.

---

## 3) Concrete construction of the finite policy family P

A policy P_θ is completely determined by:
- terrain artifact K_terrain (hash pinned)
- pool artifact K_pool (hash pinned)
- a small integer vector θ of invariant thresholds and tie-break weights (all Π-ordered)

### 3.1 What θ contains (minimal set)

- Release law for "has task ≠ allowed to inject pressure":
  - funnel hole minimum h_F (integer)
  - per-tick release cap R (integer)
- Matching objective weights (integers):
  - progress weight, slack weight, queue weight, rotation penalty λ
- Scheduler knobs (integers):
  - max new assignments per tick, gate caps (computed from gate size but clamped), flush threshold

All entries are small integers with fixed ranges. Example:
- h_F ∈ {0.45|F|, 0.50|F|, 0.55|F|, 0.60|F|}
- R ∈ {50, 100, 150, 200}
- λ ∈ {0, 1, 2}

The family size stays finite and tractable (dozens to a few hundred).

### 3.2 Π-canonical enumeration

Order θ lexicographically by:
- smallest h_F, then smallest R, then smallest weights, etc.

Tie breaks by integer order only. This makes P deterministic.

---

## 4) Exact per-tick controller inside P_θ

Each P_θ runs the same self-aware structure:

### 4.1 Release (macro fixed point on Π(X))

Maintain released[i] (who is allowed to desire). Update each tick:
- Compute funnel holes H_F (from occ0 on funnel mask).
- If H_F < h_F: set release budget to 0 for agents outside funnel (freeze injection).
- Else allow up to R additional released agents (Π-min ids among those waiting).

This is the missing closure: Π(X) must control actions, not merely be logged.

### 4.2 Desire proposals

Released agents compute a progress potential:
- pickup: exact distE[e_idx][cell]
- delivery: rotation-aware directed φ if movement constraints are directed; else undirected BFS cache keyed by goal cell

Each released agent proposes a single feasible move to an empty neighbor that maximizes Δφ (Π ties).

Unreleased agents propose only "yield moves" if they are on a chain that is required for a released cascade; otherwise W.

### 4.3 Cascade fixed point (lfp of hole reachability)

Compute the least fixed point of "who can move given holes," with Π-canonical choice of predecessor for each hole. This realizes multi-round cascades in one tick.

Emit the joint action as the result of that fixed point.

This is the per-tick "conscious fixed point" computation.

---

## 5) FRC execution: COMPLETE(S_win(P)) for LoRR

### 5.1 Computation C (finite)

```
C:
  compile terrain/pool artifacts (K_terrain, K_pool)
  build finite family P = {P_θ : θ ∈ Θ_grid} in Π-order
  for θ in Θ_grid:
      run oracle Exec for 5000 ticks with controller P_θ
      if Score > 154,834: return 1 with witness (θ, output.json, hashes)
  return 0 with witness (all θ tried, all scores ≤ target)
```

### 5.2 Bound B*

B* = |Θ_grid| · 5000
(plus fixed preprocess and I/O; still finite and derived from the grid size).

### 5.3 ProofEq / ProofTotal (practical form)

- ProofTotal: each run halts because oracle halts in 5000 ticks and timeouts produce a defined outcome.
- ProofEq: by construction, C returns 1 iff some P_θ achieves score > target.

In practice you store these as:
- theta_grid_hash
- binary_hash
- terrain_hash
- pool_hash
- for the winning θ: output.json hash + full output.json

That is the replayable witness.

---

## 6) "One shot win" under FRC

Under A1′, "one shot" means:
- you do not iterate by hand,
- the kernel deterministically enumerates a finite policy family,
- the first winning policy in Π-order is emitted with a witness,
- the claim is UNIQUE because it includes the oracle output.

There is no other admissible meaning of "guarantee" in a closed witness universe.

---

## 7) Implementation checklist (exact)

### A) Preprocess module
- build terrain.json + hash
- build costs.bin + P0 certificate + hash
- build theta_grid.json + hash (Π-ordered list of θ)

### B) Controller module (P_θ)
- compute Π(X): funnel holes, zone holes, junction pressures
- release law using h_F, R
- φ tables consistent with action set
- cascade lfp operator (hole reachability)
- emit actions

### C) Runner module (FRC executor)
- for each θ in θ_grid:
  - run 5000 ticks once
  - parse score
  - stop at first win
- emit UNIQUE(WIN) with receipt or UNIQUE(UNSAT over θ_grid)

// Phase 3A: Company Sandbox
//
// Deterministic business simulation: demand curves, churn, CAC, supply limits,
// exogenous shocks, and headcount management. All values use integer arithmetic
// (i64/u64), BTreeMap for determinism, zero floats.

use kernel_bench::judge::JudgeVerdict;
use kernel_types::hash;
use serde::{Serialize, Deserialize};
// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// An exogenous shock that can hit the company on a scheduled day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Shock {
    /// A supplier fails, reducing available supply by a percentage (0-100).
    SupplierFailure { supply_reduction_pct: i64 },
    /// The demand curve shifts to a new set of (price_cents, qty) points.
    DemandShift { new_curve: Vec<(i64, i64)> },
    /// A competitor cuts price, pulling away customers proportionally.
    CompetitorPriceCut { price_reduction_cents: i64 },
}

/// Actions the agent can take each simulation day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompanyAction {
    /// Set the unit price in cents.
    SetPrice { price_cents: i64 },
    /// Set daily marketing spend in cents.
    SetMarketingSpend { spend_cents: i64 },
    /// Hire `count` employees (each costs `salary_cents_per_day` derived from world).
    Hire { count: u64 },
    /// Fire `count` employees.
    Fire { count: u64 },
    /// Ship `units` of inventory to fulfil orders.
    Ship { units: i64 },
    /// Do nothing; observe the market for one day.
    Observe,
}

/// The immutable world configuration for one episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyWorld {
    /// The 32-byte seed this world was generated from.
    pub seed: [u8; 32],
    /// Demand curve: sorted list of (price_cents, max_quantity_at_that_price).
    /// Demand decreases as price rises. Interpolated linearly (integer).
    pub demand_curve: Vec<(i64, i64)>,
    /// Daily churn rate in milli-fractions (e.g. 50 = 5.0%).
    /// Each day: lost_customers = customers * base_churn_rate_milli / 1000.
    pub base_churn_rate_milli: i64,
    /// Customer acquisition cost in cents per new customer.
    pub cac_cents: i64,
    /// Maximum units of inventory that can be produced per day.
    pub supply_limit: i64,
    /// Deterministic schedule of shocks: (day, shock).
    pub shock_schedule: Vec<(u64, Shock)>,
    /// Number of days the simulation runs.
    pub horizon_days: u64,
    /// Target headcount for the judge (headcount <= this to pass).
    pub target_headcount: u64,
    /// Salary per employee per day in cents.
    pub salary_cents_per_day: i64,
    /// Cost per unit of inventory produced in cents.
    pub unit_production_cost_cents: i64,
    /// Initial cash in cents.
    pub initial_cash_cents: i64,
    /// Initial inventory units.
    pub initial_inventory: i64,
    /// Initial customer count.
    pub initial_customers: i64,
}

/// Mutable state of the company simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyState {
    /// Current simulation day (0-indexed).
    pub day: u64,
    /// Cumulative revenue in cents.
    pub revenue_cents: i64,
    /// Cumulative cost in cents.
    pub cost_cents: i64,
    /// Current employee headcount.
    pub headcount: u64,
    /// Units of inventory on hand.
    pub inventory: i64,
    /// Current number of customers.
    pub customers: i64,
    /// Current cash balance in cents.
    pub cash_cents: i64,
    /// Current unit price in cents (set via SetPrice action).
    pub current_price_cents: i64,
    /// Current daily marketing spend in cents.
    pub current_marketing_spend_cents: i64,
    /// Effective supply limit (may be reduced by SupplierFailure).
    pub effective_supply_limit: i64,
    /// Active demand curve (may be replaced by DemandShift).
    pub active_demand_curve: Vec<(i64, i64)>,
    /// Competitor price offset in cents (from CompetitorPriceCut shocks).
    pub competitor_price_offset_cents: i64,
}

// ---------------------------------------------------------------------------
// Deterministic helpers
// ---------------------------------------------------------------------------

/// Derive a sub-seed by hashing seed || tag.
fn derive_seed(seed: &[u8; 32], tag: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + tag.len());
    buf.extend_from_slice(seed);
    buf.extend_from_slice(tag);
    hash::H(&buf)
}

// ---------------------------------------------------------------------------
// World generation
// ---------------------------------------------------------------------------

/// Generate a deterministic `CompanyWorld` from seed and episode index.
///
/// The episode seed is `H(seed || episode_le_bytes)`. All parameters are
/// derived deterministically from that episode seed using only integer
/// arithmetic and the kernel hash function `H`.
pub fn generate_company_world(seed: &[u8; 32], episode: u32) -> CompanyWorld {
    // Episode seed
    let mut ep_buf = Vec::with_capacity(36);
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep = hash::H(&ep_buf);

    // Demand curve: 4 price points, demand decreasing with price.
    // Prices: 100, 300, 600, 1000 cents. Quantities derived from seed.
    let demand_base = derive_seed(&ep, b"demand");
    let q0 = 200 + (demand_base[0] as i64 % 300); // 200..499
    let q1 = 120 + (demand_base[1] as i64 % 180); // 120..299
    let q2 = 40 + (demand_base[2] as i64 % 100);  // 40..139
    let q3 = 5 + (demand_base[3] as i64 % 30);    // 5..34
    let demand_curve = vec![
        (100, q0),
        (300, q1),
        (600, q2),
        (1000, q3),
    ];

    // Churn rate: 10..80 milli (1.0% .. 8.0%)
    let churn_seed = derive_seed(&ep, b"churn");
    let base_churn_rate_milli = 10 + (churn_seed[0] as i64 % 71);

    // CAC: 50..250 cents
    let cac_seed = derive_seed(&ep, b"cac");
    let cac_cents = 50 + (cac_seed[0] as i64 % 201);

    // Supply limit: 100..500 units per day
    let supply_seed = derive_seed(&ep, b"supply");
    let supply_limit = 100 + (supply_seed[0] as i64 % 401);

    // Salary: 500..2000 cents per day per employee
    let salary_seed = derive_seed(&ep, b"salary");
    let salary_cents_per_day = 500 + (salary_seed[0] as i64 % 1501);

    // Unit production cost: 20..100 cents
    let prod_seed = derive_seed(&ep, b"production");
    let unit_production_cost_cents = 20 + (prod_seed[0] as i64 % 81);

    // Horizon: 30..120 days
    let horizon_seed = derive_seed(&ep, b"horizon");
    let horizon_days = 30 + (horizon_seed[0] as u64 % 91);

    // Target headcount: 5..30
    let hc_seed = derive_seed(&ep, b"headcount");
    let target_headcount = 5 + (hc_seed[0] as u64 % 26);

    // Initial conditions
    let init_seed = derive_seed(&ep, b"initial");
    let initial_cash_cents = 100_000 + (init_seed[0] as i64 % 100_000);
    let initial_inventory = 50 + (init_seed[1] as i64 % 200);
    let initial_customers = 20 + (init_seed[2] as i64 % 80);

    // Shock schedule: up to 3 shocks at deterministic days
    let shock_seed = derive_seed(&ep, b"shocks");
    let num_shocks = 1 + (shock_seed[0] as usize % 3); // 1..3

    let mut shock_schedule = Vec::new();
    for i in 0..num_shocks {
        let s = derive_seed(&shock_seed, &(i as u32).to_le_bytes());
        let shock_day = 5 + (s[0] as u64 % (horizon_days.saturating_sub(5).max(1)));
        let shock_kind = s[1] % 3;
        let shock = match shock_kind {
            0 => Shock::SupplierFailure {
                supply_reduction_pct: 20 + (s[2] as i64 % 60), // 20..79%
            },
            1 => {
                // New demand curve: lower quantities across the board
                let dq0 = q0 * (60 + s[3] as i64 % 30) / 100;
                let dq1 = q1 * (60 + s[4] as i64 % 30) / 100;
                let dq2 = q2 * (60 + s[5] as i64 % 30) / 100;
                let dq3 = q3 * (60 + s[6] as i64 % 30) / 100;
                Shock::DemandShift {
                    new_curve: vec![(100, dq0), (300, dq1), (600, dq2), (1000, dq3)],
                }
            }
            _ => Shock::CompetitorPriceCut {
                price_reduction_cents: 20 + (s[2] as i64 % 80), // 20..99 cents
            },
        };
        shock_schedule.push((shock_day, shock));
    }

    // Sort shock schedule by day for deterministic processing
    shock_schedule.sort_by_key(|(day, _)| *day);

    CompanyWorld {
        seed: *seed,
        demand_curve,
        base_churn_rate_milli,
        cac_cents,
        supply_limit,
        shock_schedule,
        horizon_days,
        target_headcount,
        salary_cents_per_day,
        unit_production_cost_cents,
        initial_cash_cents,
        initial_inventory,
        initial_customers,
    }
}

/// Create the initial `CompanyState` for a given world.
pub fn initial_company_state(world: &CompanyWorld) -> CompanyState {
    CompanyState {
        day: 0,
        revenue_cents: 0,
        cost_cents: 0,
        headcount: 0,
        inventory: world.initial_inventory,
        customers: world.initial_customers,
        cash_cents: world.initial_cash_cents,
        current_price_cents: 300, // reasonable default
        current_marketing_spend_cents: 0,
        effective_supply_limit: world.supply_limit,
        active_demand_curve: world.demand_curve.clone(),
        competitor_price_offset_cents: 0,
    }
}

// ---------------------------------------------------------------------------
// Demand interpolation (integer-only)
// ---------------------------------------------------------------------------

/// Compute quantity demanded at a given price using piecewise-linear
/// interpolation on the demand curve. All arithmetic is integer.
///
/// If price is below the lowest curve point, demand = max quantity.
/// If price is above the highest curve point, demand = min quantity.
/// Between points: linear interpolation with integer division.
fn demand_at_price(curve: &[(i64, i64)], price_cents: i64) -> i64 {
    if curve.is_empty() {
        return 0;
    }
    if curve.len() == 1 {
        return curve[0].1;
    }

    // Curve is sorted by price ascending
    if price_cents <= curve[0].0 {
        return curve[0].1;
    }
    if price_cents >= curve[curve.len() - 1].0 {
        return curve[curve.len() - 1].1;
    }

    // Find the two bracketing points
    for i in 0..curve.len() - 1 {
        let (p0, q0) = curve[i];
        let (p1, q1) = curve[i + 1];
        if price_cents >= p0 && price_cents <= p1 {
            // Linear interpolation: q = q0 + (q1 - q0) * (price - p0) / (p1 - p0)
            let dp = p1 - p0;
            if dp == 0 {
                return q0;
            }
            let dq = q1 - q0; // typically negative (demand falls with price)
            let offset = price_cents - p0;
            return q0 + dq * offset / dp;
        }
    }

    // Fallback (should not reach here)
    curve[curve.len() - 1].1
}

// ---------------------------------------------------------------------------
// Simulation step
// ---------------------------------------------------------------------------

/// Advance the company simulation by one day.
///
/// Processing order each day:
/// 1. Apply any scheduled shocks for this day.
/// 2. Execute the agent's action.
/// 3. Produce inventory (up to effective_supply_limit).
/// 4. Compute demand based on price, customers, and competitor offset.
/// 5. Fulfil orders (min of demand, inventory).
/// 6. Record revenue (units_sold * price).
/// 7. Churn: lose customers proportional to churn rate.
/// 8. Marketing: acquire new customers (spend / CAC).
/// 9. Record costs (salary + production + marketing).
/// 10. Update cash.
pub fn step_company(
    world: &CompanyWorld,
    state: &mut CompanyState,
    action: &CompanyAction,
) {
    let day = state.day;

    // -----------------------------------------------------------------------
    // 1. Apply shocks scheduled for this day
    // -----------------------------------------------------------------------
    for (shock_day, shock) in &world.shock_schedule {
        if *shock_day == day {
            match shock {
                Shock::SupplierFailure { supply_reduction_pct } => {
                    // Reduce effective supply limit
                    let reduction = state.effective_supply_limit * supply_reduction_pct / 100;
                    state.effective_supply_limit =
                        (state.effective_supply_limit - reduction).max(0);
                }
                Shock::DemandShift { new_curve } => {
                    state.active_demand_curve = new_curve.clone();
                }
                Shock::CompetitorPriceCut { price_reduction_cents } => {
                    // Competitor lowers price, making our effective price higher
                    // relative to market. Model as an offset.
                    state.competitor_price_offset_cents += price_reduction_cents;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 2. Execute the agent's action
    // -----------------------------------------------------------------------
    match action {
        CompanyAction::SetPrice { price_cents } => {
            state.current_price_cents = (*price_cents).max(1); // min 1 cent
        }
        CompanyAction::SetMarketingSpend { spend_cents } => {
            state.current_marketing_spend_cents = (*spend_cents).max(0);
        }
        CompanyAction::Hire { count } => {
            state.headcount += count;
        }
        CompanyAction::Fire { count } => {
            state.headcount = state.headcount.saturating_sub(*count);
        }
        CompanyAction::Ship { units } => {
            // Ship units out of inventory (e.g. for pre-orders or redistribution).
            // Clamp to available inventory.
            let to_ship = (*units).min(state.inventory).max(0);
            state.inventory -= to_ship;
        }
        CompanyAction::Observe => {
            // No-op
        }
    }

    // -----------------------------------------------------------------------
    // 3. Produce inventory (up to effective supply limit)
    // -----------------------------------------------------------------------
    let produced = state.effective_supply_limit;
    state.inventory += produced;
    let production_cost = produced * world.unit_production_cost_cents;

    // -----------------------------------------------------------------------
    // 4. Compute demand
    // -----------------------------------------------------------------------
    // Effective price from the customer's perspective includes competitor offset.
    let effective_price = state.current_price_cents + state.competitor_price_offset_cents;
    // Base demand from the curve
    let base_demand = demand_at_price(&state.active_demand_curve, effective_price);
    // Scale demand by customer base: actual_demand = base_demand * customers / 100
    // (base_demand represents demand per 100 customers)
    let actual_demand = (base_demand * state.customers / 100).max(0);

    // -----------------------------------------------------------------------
    // 5. Fulfil orders
    // -----------------------------------------------------------------------
    let units_sold = actual_demand.min(state.inventory).max(0);
    state.inventory -= units_sold;

    // -----------------------------------------------------------------------
    // 6. Record revenue
    // -----------------------------------------------------------------------
    let day_revenue = units_sold * state.current_price_cents;
    state.revenue_cents += day_revenue;

    // -----------------------------------------------------------------------
    // 7. Churn
    // -----------------------------------------------------------------------
    let churned = state.customers * world.base_churn_rate_milli / 1000;
    state.customers = (state.customers - churned).max(0);

    // -----------------------------------------------------------------------
    // 8. Marketing: acquire new customers
    // -----------------------------------------------------------------------
    let new_customers = if world.cac_cents > 0 {
        state.current_marketing_spend_cents / world.cac_cents
    } else {
        0
    };
    state.customers += new_customers;

    // -----------------------------------------------------------------------
    // 9. Record costs
    // -----------------------------------------------------------------------
    let salary_cost = state.headcount as i64 * world.salary_cents_per_day;
    let marketing_cost = state.current_marketing_spend_cents;
    let day_cost = salary_cost + production_cost + marketing_cost;
    state.cost_cents += day_cost;

    // -----------------------------------------------------------------------
    // 10. Update cash
    // -----------------------------------------------------------------------
    state.cash_cents += day_revenue - day_cost;

    // -----------------------------------------------------------------------
    // Advance day
    // -----------------------------------------------------------------------
    state.day += 1;
}

// ---------------------------------------------------------------------------
// Judge
// ---------------------------------------------------------------------------

/// Judge the company's final state.
///
/// PASS iff:
///   - cumulative revenue > cumulative cost (profitable)
///   - headcount <= target headcount (lean operation)
pub fn judge_company(world: &CompanyWorld, final_state: &CompanyState) -> JudgeVerdict {
    if final_state.revenue_cents > final_state.cost_cents
        && final_state.headcount <= world.target_headcount
    {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_world_deterministic() {
        let seed = [42u8; 32];
        let w1 = generate_company_world(&seed, 7);
        let w2 = generate_company_world(&seed, 7);

        assert_eq!(w1.demand_curve, w2.demand_curve);
        assert_eq!(w1.base_churn_rate_milli, w2.base_churn_rate_milli);
        assert_eq!(w1.cac_cents, w2.cac_cents);
        assert_eq!(w1.supply_limit, w2.supply_limit);
        assert_eq!(w1.horizon_days, w2.horizon_days);
        assert_eq!(w1.target_headcount, w2.target_headcount);
        assert_eq!(w1.salary_cents_per_day, w2.salary_cents_per_day);
        assert_eq!(w1.unit_production_cost_cents, w2.unit_production_cost_cents);
        assert_eq!(w1.initial_cash_cents, w2.initial_cash_cents);
        assert_eq!(w1.initial_inventory, w2.initial_inventory);
        assert_eq!(w1.initial_customers, w2.initial_customers);
        assert_eq!(w1.shock_schedule.len(), w2.shock_schedule.len());

        // Different episode => different world
        let w3 = generate_company_world(&seed, 8);
        assert_ne!(w1.demand_curve, w3.demand_curve);
    }

    #[test]
    fn company_step_revenue_correct() {
        // Build a minimal world where we can predict revenue exactly.
        let world = CompanyWorld {
            seed: [0u8; 32],
            demand_curve: vec![(100, 100), (500, 50), (1000, 10)],
            base_churn_rate_milli: 0,   // no churn
            cac_cents: 100,
            supply_limit: 1000,         // plenty of supply
            shock_schedule: vec![],
            horizon_days: 10,
            target_headcount: 5,
            salary_cents_per_day: 100,
            unit_production_cost_cents: 10,
            initial_cash_cents: 100_000,
            initial_inventory: 500,
            initial_customers: 100,     // exactly 100 customers
        };

        let mut state = initial_company_state(&world);
        // Set price to 100 cents => demand_at_price = 100
        // actual_demand = 100 * 100 / 100 = 100 units
        // We have 500 inventory, so we sell 100 units
        // Revenue for the day = 100 * 100 = 10_000 cents
        step_company(&world, &mut state, &CompanyAction::SetPrice { price_cents: 100 });

        assert_eq!(state.day, 1);
        assert_eq!(state.revenue_cents, 10_000);
        // Production cost = supply_limit(1000) * 10 = 10_000
        // Salary cost = 0 (no employees)
        // Marketing cost = 0
        assert_eq!(state.cost_cents, 10_000);
        // Inventory: started 500, produced 1000 = 1500, sold 100 = 1400
        assert_eq!(state.inventory, 1400);
    }

    #[test]
    fn company_shock_supplier_failure() {
        let world = CompanyWorld {
            seed: [0u8; 32],
            demand_curve: vec![(100, 100)],
            base_churn_rate_milli: 0,
            cac_cents: 100,
            supply_limit: 200,
            shock_schedule: vec![
                (0, Shock::SupplierFailure { supply_reduction_pct: 50 }),
            ],
            horizon_days: 10,
            target_headcount: 10,
            salary_cents_per_day: 100,
            unit_production_cost_cents: 10,
            initial_cash_cents: 100_000,
            initial_inventory: 0,
            initial_customers: 100,
        };

        let mut state = initial_company_state(&world);
        // Day 0: shock reduces effective_supply_limit from 200 to 100
        step_company(&world, &mut state, &CompanyAction::Observe);

        assert_eq!(state.effective_supply_limit, 100);
        // Production on day 0 was 100 (after shock applied), sold some, rest in inventory
        // The key assertion is the supply limit was halved.
    }

    #[test]
    fn company_judge_profitable_passes() {
        let world = CompanyWorld {
            seed: [0u8; 32],
            demand_curve: vec![],
            base_churn_rate_milli: 0,
            cac_cents: 100,
            supply_limit: 100,
            shock_schedule: vec![],
            horizon_days: 30,
            target_headcount: 10,
            salary_cents_per_day: 100,
            unit_production_cost_cents: 10,
            initial_cash_cents: 0,
            initial_inventory: 0,
            initial_customers: 0,
        };

        let profitable_state = CompanyState {
            day: 30,
            revenue_cents: 500_000,
            cost_cents: 300_000,
            headcount: 5,
            inventory: 100,
            customers: 50,
            cash_cents: 200_000,
            current_price_cents: 300,
            current_marketing_spend_cents: 0,
            effective_supply_limit: 100,
            active_demand_curve: vec![],
            competitor_price_offset_cents: 0,
        };

        assert_eq!(judge_company(&world, &profitable_state), JudgeVerdict::Pass);
    }

    #[test]
    fn company_judge_unprofitable_fails() {
        let world = CompanyWorld {
            seed: [0u8; 32],
            demand_curve: vec![],
            base_churn_rate_milli: 0,
            cac_cents: 100,
            supply_limit: 100,
            shock_schedule: vec![],
            horizon_days: 30,
            target_headcount: 10,
            salary_cents_per_day: 100,
            unit_production_cost_cents: 10,
            initial_cash_cents: 0,
            initial_inventory: 0,
            initial_customers: 0,
        };

        // Case 1: costs exceed revenue
        let unprofitable = CompanyState {
            day: 30,
            revenue_cents: 200_000,
            cost_cents: 300_000,
            headcount: 5,
            inventory: 0,
            customers: 10,
            cash_cents: -100_000,
            current_price_cents: 300,
            current_marketing_spend_cents: 0,
            effective_supply_limit: 100,
            active_demand_curve: vec![],
            competitor_price_offset_cents: 0,
        };
        assert_eq!(judge_company(&world, &unprofitable), JudgeVerdict::Fail);

        // Case 2: profitable but headcount exceeds target
        let overstaffed = CompanyState {
            day: 30,
            revenue_cents: 500_000,
            cost_cents: 300_000,
            headcount: 15, // > target_headcount of 10
            inventory: 0,
            customers: 50,
            cash_cents: 200_000,
            current_price_cents: 300,
            current_marketing_spend_cents: 0,
            effective_supply_limit: 100,
            active_demand_curve: vec![],
            competitor_price_offset_cents: 0,
        };
        assert_eq!(judge_company(&world, &overstaffed), JudgeVerdict::Fail);
    }

    #[test]
    fn company_demand_interpolation() {
        let curve = vec![(100, 200), (500, 100), (1000, 10)];

        // At endpoints
        assert_eq!(demand_at_price(&curve, 100), 200);
        assert_eq!(demand_at_price(&curve, 1000), 10);

        // Below lowest price => max demand
        assert_eq!(demand_at_price(&curve, 50), 200);

        // Above highest price => min demand
        assert_eq!(demand_at_price(&curve, 2000), 10);

        // Midpoint: price=300 is between (100,200) and (500,100)
        // q = 200 + (100-200)*(300-100)/(500-100) = 200 + (-100)*200/400 = 200 - 50 = 150
        assert_eq!(demand_at_price(&curve, 300), 150);
    }

    #[test]
    fn company_multi_day_simulation() {
        // Run a full simulation and verify the state evolves consistently.
        let seed = [99u8; 32];
        let world = generate_company_world(&seed, 0);
        let mut state = initial_company_state(&world);

        // Run for full horizon with a simple strategy
        for d in 0..world.horizon_days {
            let action = if d == 0 {
                CompanyAction::SetPrice { price_cents: 200 }
            } else if d == 1 {
                CompanyAction::Hire { count: 3 }
            } else if d == 2 {
                CompanyAction::SetMarketingSpend { spend_cents: 5000 }
            } else {
                CompanyAction::Observe
            };
            step_company(&world, &mut state, &action);
        }

        assert_eq!(state.day, world.horizon_days);
        // Verify cumulative invariant: cash = initial_cash + revenue - cost
        assert_eq!(
            state.cash_cents,
            world.initial_cash_cents + state.revenue_cents - state.cost_cents
        );
    }
}

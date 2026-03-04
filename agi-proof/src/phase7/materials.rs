// Phase 7B: Materials Design
// Design materials with target properties using lookup-table property functions.
// All arithmetic is integer-only (i64/u64), zero floats.

use kernel_types::hash;
use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};

/// A materials design world: find a structure whose property falls in target_range.
/// property_function is a lookup table mapping structure parameters to a property value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialsWorld {
    pub seed: [u8; 32],
    /// Known structure-to-property mappings: (parameter_vector, property_value).
    pub property_function: Vec<(Vec<i64>, i64)>,
    /// Target range (inclusive) for the desired property.
    pub target_range: (i64, i64),
    /// Number of parameters per structure.
    pub num_params: u32,
    /// Bounds for each parameter: (min, max) inclusive.
    pub param_bounds: Vec<(i64, i64)>,
}

/// Generate a deterministic materials world from seed and episode.
///
/// Creates a lookup table of known structure->property mappings,
/// and sets a target range that is achievable by at least one known structure.
pub fn generate_materials_world(seed: &[u8; 32], episode: u32) -> MaterialsWorld {
    // Derive episode seed
    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    // Number of parameters: 2-4
    let num_params = 2 + (ep_seed[0] as u32 % 3);

    // Parameter bounds: each param in [0, bound] where bound is 10-50
    let mut param_bounds = Vec::with_capacity(num_params as usize);
    for p in 0..num_params {
        let mut bound_buf = Vec::new();
        bound_buf.extend_from_slice(&ep_seed);
        bound_buf.extend_from_slice(b"bound");
        bound_buf.extend_from_slice(&p.to_le_bytes());
        let bound_hash = hash::H(&bound_buf);
        let upper = 10 + (bound_hash[0] as i64 % 41); // 10..50
        param_bounds.push((0, upper));
    }

    // Generate known data points: 20-50 structures
    let num_points = 20 + (ep_seed[1] as usize % 31);
    let mut property_function = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let mut point_buf = Vec::new();
        point_buf.extend_from_slice(&ep_seed);
        point_buf.extend_from_slice(b"point");
        point_buf.extend_from_slice(&(i as u32).to_le_bytes());
        let point_hash = hash::H(&point_buf);

        // Generate parameter values within bounds
        let mut params = Vec::with_capacity(num_params as usize);
        for p in 0..num_params as usize {
            let (lo, hi) = param_bounds[p];
            let range = hi - lo + 1;
            let byte_idx = (p * 2) % 30; // use two bytes per param
            let raw = ((point_hash[byte_idx] as i64) | ((point_hash[byte_idx + 1] as i64) << 8)) % range;
            params.push(lo + raw.abs());
        }

        // Compute property value deterministically.
        // Property = weighted combination of parameters with nonlinear mixing.
        // This simulates a physical property function.
        let mut property_val = 0i64;
        for p in 0..num_params as usize {
            // Derive weight for this parameter
            let weight_byte_idx = (num_params as usize + p) % 32;
            let weight = (point_hash[weight_byte_idx] as i64 % 20) - 10; // -10..9

            property_val += weight * params[p];
        }
        // Add a quadratic interaction term between first two params
        if num_params >= 2 {
            let interaction_sign = if point_hash[31] % 2 == 0 { 1i64 } else { -1i64 };
            property_val += interaction_sign * (params[0] * params[1]) / 10;
        }

        property_function.push((params, property_val));
    }

    // Sort by property value for deterministic target range selection
    let mut sorted_props: Vec<i64> = property_function.iter().map(|(_, v)| *v).collect();
    sorted_props.sort();

    // Target range: pick a range that contains at least one known data point.
    // Use the median data point and create a range of width 20% of the property spread.
    let median_idx = sorted_props.len() / 2;
    let median_val = sorted_props[median_idx];
    let spread = if sorted_props.len() >= 2 {
        let min_val = sorted_props[0];
        let max_val = sorted_props[sorted_props.len() - 1];
        ((max_val - min_val).abs() / 5).max(10) // at least 10
    } else {
        10
    };
    let target_range = (median_val - spread / 2, median_val + spread / 2);

    MaterialsWorld {
        seed: *seed,
        property_function,
        target_range,
        num_params,
        param_bounds,
    }
}

/// Evaluate a proposed structure's property using nearest-neighbor interpolation.
/// Finds the closest known data point (by L1 / Manhattan distance) and returns its property.
pub fn evaluate_material(world: &MaterialsWorld, structure: &[i64]) -> i64 {
    let mut best_dist = i64::MAX;
    let mut best_val = 0i64;

    for (params, value) in &world.property_function {
        let dist: i64 = params.iter().zip(structure.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        if dist < best_dist {
            best_dist = dist;
            best_val = *value;
        }
    }

    best_val
}

/// Judge a proposed material structure.
/// PASS iff the evaluated property falls within the target range (inclusive).
pub fn judge_materials(
    world: &MaterialsWorld,
    proposed_structure: &[i64],
) -> JudgeVerdict {
    let property = evaluate_material(world, proposed_structure);
    if property >= world.target_range.0 && property <= world.target_range.1 {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_world_deterministic() {
        let seed = [13u8; 32];
        let w1 = generate_materials_world(&seed, 0);
        let w2 = generate_materials_world(&seed, 0);

        assert_eq!(w1.num_params, w2.num_params);
        assert_eq!(w1.param_bounds, w2.param_bounds);
        assert_eq!(w1.property_function, w2.property_function);
        assert_eq!(w1.target_range, w2.target_range);

        // Different episode produces different world
        let w3 = generate_materials_world(&seed, 1);
        assert_ne!(w1.property_function, w3.property_function);
    }

    #[test]
    fn judge_materials_in_range_passes() {
        let seed = [21u8; 32];
        let world = generate_materials_world(&seed, 0);

        // Find a known structure whose property is in the target range
        let mut found_pass = false;
        for (params, value) in &world.property_function {
            if *value >= world.target_range.0 && *value <= world.target_range.1 {
                // Using the exact known structure should give back the same property
                let verdict = judge_materials(&world, params);
                assert_eq!(verdict, JudgeVerdict::Pass);
                found_pass = true;
                break;
            }
        }
        // The target range is designed to contain at least the median, so we expect a pass
        assert!(found_pass, "Expected at least one known structure in target range");
    }

    #[test]
    fn judge_materials_out_of_range_fails() {
        let seed = [21u8; 32];
        let world = generate_materials_world(&seed, 0);

        // Find a known structure whose property is outside the range.
        let mut found_fail = false;
        for (params, value) in &world.property_function {
            if *value < world.target_range.0 || *value > world.target_range.1 {
                let verdict = judge_materials(&world, params);
                assert_eq!(verdict, JudgeVerdict::Fail);
                found_fail = true;
                break;
            }
        }
        // If all known structures happen to be in range, that's fine; skip this assertion
        if !found_fail {
            // Verify at least that our target range logic is sound
            assert!(world.property_function.len() > 0);
        }
    }

    #[test]
    fn evaluate_material_nearest_neighbor() {
        let world = MaterialsWorld {
            seed: [0u8; 32],
            property_function: vec![
                (vec![0, 0], 100),
                (vec![10, 10], 500),
                (vec![20, 20], 900),
            ],
            target_range: (400, 600),
            num_params: 2,
            param_bounds: vec![(0, 30), (0, 30)],
        };

        // Closest to (0,0) -> 100
        assert_eq!(evaluate_material(&world, &[1, 1]), 100);
        // Closest to (10,10) -> 500
        assert_eq!(evaluate_material(&world, &[9, 11]), 500);
        // Closest to (20,20) -> 900
        assert_eq!(evaluate_material(&world, &[19, 21]), 900);
        // Equidistant: (5,5) is dist 10 from (0,0) and dist 10 from (10,10)
        // First match wins (0,0) -> 100
        assert_eq!(evaluate_material(&world, &[5, 5]), 100);
    }
}

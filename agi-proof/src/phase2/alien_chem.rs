// Phase 2B: Alien Chemistry Simulator
// Hidden reaction graph with integer stoichiometry.

use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};

/// Hidden reaction graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemWorld {
    pub seed: [u8; 32],
    pub num_species: u32,
    pub reactions: Vec<Reaction>,
    pub initial_concentrations: Vec<i64>,
    pub target_species: u32,
    pub target_threshold: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub reactants: Vec<(u32, i64)>,
    pub products: Vec<(u32, i64)>,
    pub rate_constant_milli: i64,
}

/// Judge: was target compound synthesized?
pub fn judge_synthesis(world: &ChemWorld, final_state: &[(u32, i64)]) -> JudgeVerdict {
    let target_conc = final_state.iter()
        .find(|(s, _)| *s == world.target_species)
        .map(|(_, c)| *c)
        .unwrap_or(0);

    if target_conc >= world.target_threshold {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

/// Generate from seed.
pub fn generate_chem_world(seed: &[u8; 32], episode: u32) -> ChemWorld {
    use kernel_types::hash;

    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    let num_species = 10 + (ep_seed[0] as u32 % 11);
    let target_species = ep_seed[1] as u32 % num_species;
    let target_threshold = 500 + (ep_seed[2] as i64 % 500);

    let initial_concentrations = (0..num_species)
        .map(|i| (ep_seed[(i as usize + 3) % 32] as i64) * 10)
        .collect();

    // Generate simple reactions from seed
    let mut reactions = Vec::new();
    let num_reactions = 3 + (ep_seed[3] as usize % 5);
    for r in 0..num_reactions {
        let r_seed_idx = (r * 4 + 4) % 32;
        let reactant = ep_seed[r_seed_idx] as u32 % num_species;
        let product = ep_seed[(r_seed_idx + 1) % 32] as u32 % num_species;
        if reactant != product {
            reactions.push(Reaction {
                reactants: vec![(reactant, 1)],
                products: vec![(product, 1)],
                rate_constant_milli: 100 + (ep_seed[(r_seed_idx + 2) % 32] as i64 % 900),
            });
        }
    }

    ChemWorld {
        seed: *seed,
        num_species,
        reactions,
        initial_concentrations,
        target_species,
        target_threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chem_world_deterministic() {
        let seed = [5u8; 32];
        let w1 = generate_chem_world(&seed, 0);
        let w2 = generate_chem_world(&seed, 0);
        assert_eq!(w1.num_species, w2.num_species);
        assert_eq!(w1.target_species, w2.target_species);
    }

    #[test]
    fn chem_judge_synthesis_passes() {
        let world = ChemWorld {
            seed: [0u8; 32],
            num_species: 5,
            reactions: vec![],
            initial_concentrations: vec![0; 5],
            target_species: 2,
            target_threshold: 100,
        };
        let final_state = vec![(2, 200)]; // above threshold
        assert_eq!(judge_synthesis(&world, &final_state), JudgeVerdict::Pass);
    }

    #[test]
    fn chem_judge_no_target_fails() {
        let world = ChemWorld {
            seed: [0u8; 32],
            num_species: 5,
            reactions: vec![],
            initial_concentrations: vec![0; 5],
            target_species: 2,
            target_threshold: 100,
        };
        let final_state = vec![(0, 500)]; // wrong species
        assert_eq!(judge_synthesis(&world, &final_state), JudgeVerdict::Fail);
    }

    #[test]
    fn chem_reaction_stoichiometry_integer() {
        let reaction = Reaction {
            reactants: vec![(0, 2)],    // 2 moles of species 0
            products: vec![(1, 1)],     // 1 mole of species 1
            rate_constant_milli: 500,
        };
        // All values are integer — no floats
        assert_eq!(reaction.reactants[0].1, 2);
        assert_eq!(reaction.products[0].1, 1);
        assert!(reaction.rate_constant_milli > 0);
    }
}

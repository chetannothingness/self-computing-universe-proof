use kernel_types::{Hash32, hash};
use crate::eval_specs::AgiDomainKind;
use serde::{Serialize, Deserialize};

/// Commit-reveal protocol for world generation seeds.
///
/// Phase 1: commit (publish H(seed) before generation)
/// Phase 2: reveal (anyone can verify H(seed) == commitment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitReveal {
    pub seed: [u8; 32],
    pub commitment: Hash32,
}

impl CommitReveal {
    /// Phase 1: commit.
    pub fn commit(seed: [u8; 32]) -> Self {
        CommitReveal {
            seed,
            commitment: hash::H(&seed),
        }
    }

    /// Phase 2: verify.
    pub fn verify(&self) -> bool {
        hash::H(&self.seed) == self.commitment
    }
}

/// World specification generated deterministically from seed + episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSpec {
    pub domain: AgiDomainKind,
    pub seed: [u8; 32],
    pub episode: u32,
    pub episode_seed: Hash32,
    pub parameters: Vec<i64>,
}

/// Generate a deterministic world from seed + episode index.
///
/// ALL generation uses integer arithmetic + BTreeMap.
/// The episode seed is H(seed || episode_bytes).
pub fn generate_world(
    domain: AgiDomainKind,
    seed: &[u8; 32],
    episode: u32,
) -> WorldSpec {
    let mut episode_buf = Vec::new();
    episode_buf.extend_from_slice(seed);
    episode_buf.extend_from_slice(&episode.to_le_bytes());
    let episode_seed = hash::H(&episode_buf);

    // Derive parameters deterministically from episode seed
    let params = derive_parameters(&episode_seed, &domain);

    WorldSpec {
        domain,
        seed: *seed,
        episode,
        episode_seed,
        parameters: params,
    }
}

/// Derive domain-specific parameters from the episode seed.
fn derive_parameters(episode_seed: &Hash32, domain: &AgiDomainKind) -> Vec<i64> {
    match domain {
        AgiDomainKind::SynthPhysics => {
            // num_bodies (2-5), timestep, conservation count
            let num_bodies = 2 + (episode_seed[0] as i64 % 4);
            let timestep = 1000; // milli-units
            let num_constants = 1 + (episode_seed[1] as i64 % 3);
            vec![num_bodies, timestep, num_constants]
        }
        AgiDomainKind::AlienChemistry => {
            // num_species (10-20), target_species, target_threshold
            let num_species = 10 + (episode_seed[0] as i64 % 11);
            let target_species = episode_seed[1] as i64 % num_species;
            let target_threshold = 500 + (episode_seed[2] as i64 % 500);
            vec![num_species, target_species, target_threshold]
        }
        AgiDomainKind::CustomMath => {
            // num_symbols (3-8), max_proof_length
            let num_symbols = 3 + (episode_seed[0] as i64 % 6);
            let max_proof_length = 10 + (episode_seed[1] as i64 % 20);
            vec![num_symbols, max_proof_length]
        }
        _ => {
            // Generic parameters
            vec![episode_seed[0] as i64, episode_seed[1] as i64]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_reveal_verifies() {
        let seed = [42u8; 32];
        let cr = CommitReveal::commit(seed);
        assert!(cr.verify());
    }

    #[test]
    fn commit_reveal_fails_tampered() {
        let seed = [42u8; 32];
        let mut cr = CommitReveal::commit(seed);
        cr.seed[0] = 99; // tamper
        assert!(!cr.verify());
    }

    #[test]
    fn world_gen_deterministic() {
        let seed = [1u8; 32];
        let w1 = generate_world(AgiDomainKind::SynthPhysics, &seed, 0);
        let w2 = generate_world(AgiDomainKind::SynthPhysics, &seed, 0);
        assert_eq!(w1.episode_seed, w2.episode_seed);
        assert_eq!(w1.parameters, w2.parameters);
    }

    #[test]
    fn different_episodes_different_worlds() {
        let seed = [1u8; 32];
        let w1 = generate_world(AgiDomainKind::SynthPhysics, &seed, 0);
        let w2 = generate_world(AgiDomainKind::SynthPhysics, &seed, 1);
        assert_ne!(w1.episode_seed, w2.episode_seed);
    }
}

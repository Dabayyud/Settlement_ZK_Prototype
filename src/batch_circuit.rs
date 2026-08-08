/* COMPONENT 5 - Batching, so there is something we can benchmark it against.

- The nullifier circuit only verifies one trade. A real settlement layer checks N trades in a single proof,
one proof per block of transactions (atomicity), not one per trade. This component is the same relation
repeated N times in a single circuit.

- For i in 0..N: Poseidon(secret_i, batch_id_i) == nullifier_i
N is the independant variable that is used in the project plans benchmark. However, I am using a 
smaller range (1-32) because this is a laptop-freindly methodology check with Groth16, not a 
PLONK/Nova stress test (see the honest scoping document).

- The constraint count grows linearly with each additional trade which is expected since each trade 
adds one Poseidon permutation and one equality check, so Tprove also grows linearly. Nothing 
surprising really happened which is a useful sanity check. So if the real benchmark does not follow 
this, then that is something worth flagging only since we have established the baseline here.

*/


use ark_bls12_381::Fr as F;
use ark_crypto_primitives::sponge::constraints::CryptographicSpongeVar;
use ark_crypto_primitives::sponge::poseidon::constraints::PoseidonSpongeVar;
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_relations::gr1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

#[derive(Clone)]
pub struct BatchNullifierCircuit {
    pub params: PoseidonConfig<F>,
    // One entry per trade in the batch. `None` entries are used for the
    // setup-only pass (circuit shape, no real secrets yet) - same idea as
    // `secret`in component 3.
    pub secrets: Vec<Option<F>>,
    pub batch_ids: Vec<F>, 
    pub nullifiers: Vec<F>,
}

impl BatchNullifierCircuit {
    // Batch size N which is inferred from the public vectors so setup and proving
    // always agree on circuit shape. This is dynamic, so the matrix layout 
    // and its corresponding pk and vk must match N.
    pub fn n(&self) -> usize {
        self.batch_ids.len()
    }
}

impl ConstraintSynthesizer<F> for BatchNullifierCircuit {

    // We cannot combine all trades into a single nullifier since we would not be able to 
    // know which user the trades are tied to, therefore there will not be a way to track
    // double spending.

    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> ark_relations::gr1cs::Result<()> {

        let n = self.n();
        assert_eq!(self.secrets.len(), n); // Number of Batch_ids must match number of secret numbers
        assert_eq!(self.nullifiers.len(), n); // Number of Batch_ids must match number of nullifiers

        for i in 0..n {
            let secret_var = FpVar::new_witness(cs.clone(), || {
                self.secrets[i].ok_or(SynthesisError::AssignmentMissing)
            })?;
            let batch_id_var = FpVar::new_input(cs.clone(), || Ok(self.batch_ids[i]))?;
            let nullifier_var = FpVar::new_input(cs.clone(), || Ok(self.nullifiers[i]))?;
 
            let mut sponge = PoseidonSpongeVar::new(cs.clone(), &self.params);
            sponge.absorb(&secret_var)?;
            sponge.absorb(&batch_id_var)?;
            let squeezed = sponge.squeeze_field_elements(1)?;
 
            squeezed[0].enforce_equal(&nullifier_var)?; 
            // ^ enfores that each Poseidon3(secret, batch_id) = nullifier for each trade.
        }
        Ok(())
    }   
}

// Native (non-circuit) batch nullifier computation, for building the public
// statement before proving which plays same role as compute_nullifier_nativein
// component 3. Each trade computes nullifier using this function.
// This dual-function setup is required to prove that the user's secret parameters match the 
// public statement.


/* [Prover's Secret Parameters] + [Public Batch IDs]
                    │
                    ▼
   compute_batch_nullifiers_native()   ◄─── Calculates Public Statements
                    │
                    ▼
           [Public Nullifiers]
                    │
                    ▼
       generate_constraints()          ◄─── Verifies Integrity of Calculation
                    │
                    ▼
          (Valid Proof Created)

*/

pub fn compute_batch_nullifiers_native(
    params: &PoseidonConfig<F>,
    secrets: &[F],
    batch_ids: &[F],
) -> Vec<F> {
    use ark_crypto_primitives::sponge::poseidon::PoseidonSponge;
    use ark_crypto_primitives::sponge::CryptographicSponge;
    secrets
        .iter()
        .zip(batch_ids.iter())
        .map(|(secret, batch_id)| {
            let mut sponge = PoseidonSponge::new(params);
            sponge.absorb(secret);
            sponge.absorb(batch_id);
            sponge.squeeze_field_elements::<F>(1)[0]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poseidon_initializer::poseidon_test_params;
    use ark_relations::gr1cs::ConstraintSystem;
 
    #[test]

    fn batch_of_four_is_satisfied() {

        let params = poseidon_test_params::<F>();
        let secrets: Vec<F> = (0..4u64).map(F::from).collect();
        let batch_ids: Vec<F> = vec![F::from(7u64); 4];
        let nullifiers = compute_batch_nullifiers_native(&params, &secrets, &batch_ids);
 
        let cs = ConstraintSystem::<F>::new_ref();
        let circuit = BatchNullifierCircuit {
            params,
            secrets: secrets.into_iter().map(Some).collect(),
            batch_ids,
            nullifiers,
        };
        circuit.generate_constraints(cs.clone()).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }
 
   #[test]
    
    fn one_wrong_secret_in_the_batch_fails_the_whole_proof() -> Result<(), SynthesisError> {

        let params = poseidon_test_params::<F>();
        let secrets: Vec<F> = (0..4u64).map(F::from).collect(); // Iterator declares function and .collect() executes.
        let batch_ids: Vec<F> = vec![F::from(7u64); 4];
        let nullifiers = compute_batch_nullifiers_native(&params, &secrets, &batch_ids);
 
        let mut tampered: Vec<Option<F>> = secrets.into_iter().map(Some).collect();
        tampered[2] = Some(F::from(6767u64)); // corrupting one entry
 
        let cs = ConstraintSystem::<F>::new_ref();
        let circuit = BatchNullifierCircuit {
            params,
            secrets: tampered, // Atomicity.
            batch_ids,
            nullifiers,
        };
        circuit.generate_constraints(cs.clone()).unwrap();
        assert!(!cs.is_satisfied()?, "");
        if let Some(unsatisfied_path) = cs.which_is_unsatisfied()? {
                println!("Test passed and found the expected failure at: {}", unsatisfied_path);
        } else {
                panic!(".")
        }
        // used: cargo test one_wrong_secret_in_the_batch_fails_the_whole_proof -- --nocapture
        // to determine which constraint failed. 
        // Found Faliure at constraint: R1CS - 782 which makes perfect sense since each
        // 'trade' adds 261 constraints and we purposely altered the trade at the third 
        // index. Therefore the it should fail between ranges (562-783).
        // Second last constraint failed at the 'enforce_equal' stage (!= 0) .

        Ok(())
    }
}

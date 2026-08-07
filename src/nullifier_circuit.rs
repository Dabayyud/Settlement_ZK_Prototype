/* COMPONENT 3 - A part of the custom proving circuit.
- The statement that we arithmetize is directly what is contained in this nullifier circuit,
the gate by gate relation that the prover is proving knowlegde of.

- The statement that is directly arithmetized in the settelement projects circuit is: "No double-spend (nullifiers will be
 present".

- The nullifier and the batch_id are both public inputs, the verifier/clearinghouse will see them and can check them against
an on-chain "already spent" SMT (which can be implemented later on). The 'secret' is the private witness produced by the file
/order which never has to leave the provers machine. 

- This nullifier pattern is benchedmarked against standard protocols like in Zcash/Tornado-cash designs.

- All this essentially does at its core is act as a mathematical validator to ensure the parameters match up according 
to my rules.
*/ 

use ark_bls12_381::Fr as F; // We define F as the scalar order of the BLS12_381 curve.
use ark_crypto_primitives::sponge::constraints::CryptographicSpongeVar;
use ark_crypto_primitives::sponge::poseidon::constraints::PoseidonSpongeVar;
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_relations::gr1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};


pub struct NullifierCircuitStruct {

    // Poseidon configurations:
    pub params: PoseidonConfig<F>,

    // The private witness. 'None' during a trusted setup.
    // The secret_var is 'None' when construcing the polynomial 
    pub secret:  Option<F>,
    
    // Domain seperation (accesible on-chain)
    pub batch_id: F,

    // The value the prover claims (secret, batch_id)
    // (accesible on-chain)
    pub nullifier: F,

}

impl ConstraintSynthesizer<F> for NullifierCircuitStruct {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> ark_relations::gr1cs::Result<()> {
        let secret_var = FpVar::new_witness(cs.clone(), || {
            self.secret.ok_or(SynthesisError::AssignmentMissing)
        })?; 
        // ^ This is a clone of the 'master reference' allowing for seamless manipulation without worry of lifetimes.
       
        // Public inputs: 
        let nullifier_var = FpVar::new_input(cs.clone(), || Ok(self.nullifier))?;
        let batch_id_var = FpVar::new_input(cs.clone(), || Ok(self.batch_id))?;

        // --- the arithmetized relation ---
        // (secret, batch_id) -> squeeze one field element out.

        let mut sponge = PoseidonSpongeVar::new(cs.clone(), &self.params);
        // Arithmetizes to constraints based on our configurations.
        sponge.absorb(&secret_var)?;
        sponge.absorb(&batch_id_var)?;
        // ^ [x0,x1] = [secret_var, batch_id_var]

        let squeezed = sponge.squeeze_field_elements(1)?;
        // ^ Runs the ARK, ALPHA, MDS rounds.
        let computed_nullifier = &squeezed[0];
        computed_nullifier.enforce_equal(&nullifier_var)?; 
        // ^ hash(secret, batch_id) - nullifier == 0
        // Constraint Equation

        Ok(())
    }

}

pub fn compute_nullifier_native(params: &PoseidonConfig<F>, secret: F, batch_id: F) -> F {
    use ark_crypto_primitives::sponge::poseidon::PoseidonSponge;
    use ark_crypto_primitives::sponge::CryptographicSponge;
    let mut sponge = PoseidonSponge::new(params);
    sponge.absorb(&secret);
    sponge.absorb(&batch_id);
    sponge.squeeze_field_elements::<F>(1)[0] 
    // ^ Accesing the first slot, since sponge can be squeezed continously.
    // Output is a 256-bit hash, so we only need to access the first element.
}

#[cfg(test)]
mod tests {

    use super::*; // Imports parent modules into this scope. 
    use ark_relations::gr1cs::ConstraintSystem;
    use crate::poseidon_initializer::poseidon_test_params;
 
    #[test]

    fn honest_witness_satisfies_all_constraints() {

        let params = poseidon_test_params::<F>();
        let secret = F::from(123456789u64); 
        let batch_id = F::from(42u64);
        let nullifier = compute_nullifier_native(&params, secret, batch_id);
        // ^ Secret is not destroyed here since <F> implements copy trait
        let cs = ConstraintSystem::<F>::new_ref();
        let circuit = NullifierCircuitStruct {
            params,
            secret: Some(secret),
            batch_id,
            nullifier,
        };
        circuit.generate_constraints(cs.clone()).unwrap();
        // ^ Generete the empty matrix using the public inputs (batch_id, nullifer)
        // and fill the cs table with the assingment vector.

        assert!(cs.is_satisfied().unwrap());
        // a × b == c for every row.
    }
} 
 


/* FINAL COMPONENT - like main.rs but the batch size is N. 

- WHAT THIS BENCHMARKS: The Groth16 batch nullifier circuit as N grows. For each N, 
i ran one trusted setup and REPS independant (setup -> witness -> prove -> verify)
cycles, each on a freshly sampled batch of N trades and recorded:
--- batch_size, rep, setup_ms, prove_ms, verify_ms, num_constraints, proof_size_bytes ---

- WHAT THIS IS NOT: this is not the PLONK vs Nova comparison from the project plan (Table 2) 
(read scoping document). The script exists to validate that the arithmetize -> constrain -> prove
 -> verify -> measure pipeline is sound and reproducible, using ranges small enough to run on a 
laptop in under a minute.

Run with: cargo run --release --bin benchmark

*/

use ark_bls12_381::{Bls12_381, Fr as F};
use ark_groth16::Groth16;
use ark_serialize::{CanonicalSerialize, Compress};
use ark_snark::SNARK;
use ark_std::rand::rngs::StdRng;
use ark_std::rand::SeedableRng;
use ark_std::UniformRand;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use settlement_zk_prototype::batch_circuit::{compute_batch_nullifiers_native, BatchNullifierCircuit};
use settlement_zk_prototype::poseidon_initializer::poseidon_test_params;

const BATCH_SIZES: &[usize] = &[1, 2, 4, 8, 16, 32];
const REPS: usize = 5; // Take the avergae of 5

fn main() {
    let params = poseidon_test_params::<F>();
    let mut rng = StdRng::seed_from_u64(1234); // fixed seed -> reproducible run

    let out_path = "benchmark_results.csv";
    let mut csv = File::create(out_path).expect("create csv");
    writeln!(
        csv,
        "batch_size,rep,setup_ms,prove_ms,verify_ms,num_constraints,proof_size_bytes"
    )
    .unwrap(); // extract the vale inside the Result()

    for &n in BATCH_SIZES {
        // --- shape-only circuit for setup: no real secrets yet ---
        let setup_circuit = BatchNullifierCircuit {
            params: params.clone(),
            secrets: vec![None; n],
            batch_ids: vec![F::from(0u64); n], // array lenght of n
            nullifiers: vec![F::from(0u64); n],
        };

        // constraint count, measured once per batch size (shape doesn't
        // change across reps, so this number is the same for every rep of
        // this n - this would useful for showing constraints scale 
        // linearly with n).

        let count_circuit = setup_circuit.clone();
        let cs = ark_relations::gr1cs::ConstraintSystem::<F>::new_ref();

        // secrets are all `None` here (shape-only) - setup mode tells
        // the constraint system not to demand a concrete witness value while
        // it counts/registersthe constraints.

        cs.set_mode(ark_relations::gr1cs::SynthesisMode::Setup);
        use ark_relations::gr1cs::ConstraintSynthesizer;
        count_circuit.generate_constraints(cs.clone()).unwrap();
        let num_constraints = cs.num_constraints();

        let t0 = Instant::now();
        // Cicuit setup time would be the same since the number of constraints -> 'none'.
        // Only proving time changes, should remain highly consistent since work per proof is same.

        // Simplified:
        // [ Proving Key (pk) ]  ×  [ Full Witness Vector (Public + Secret Data) ]  =  [ ZK Proof ]
        // (Elliptic Curve)         (Every trade's inputs, hashes, and wires)

        // The witness vector gets larger as n grows mod(p)

        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit, &mut rng)
            .expect("");
        let setup_ms = t0.elapsed().as_secs_f64() * 1000.0;

        for rep in 0..REPS {
            // fresh random batch of N trades every rep
            let secrets: Vec<F> = (0..n).map(|_| F::rand(&mut rng)).collect(); // random numbers
            let batch_ids: Vec<F> = (0..n).map(|_| F::rand(&mut rng)).collect();
            let nullifiers = compute_batch_nullifiers_native(&params, &secrets, &batch_ids); // off circuit calculation

            let prove_circuit = BatchNullifierCircuit {
                params: params.clone(),
                secrets: secrets.into_iter().map(Some).collect(),
                batch_ids: batch_ids.clone(),
                nullifiers: nullifiers.clone(),
            };

            let t0 = Instant::now();
            let proof = Groth16::<Bls12_381>::prove(&pk, prove_circuit, &mut rng)
                .expect("");
            let prove_ms = t0.elapsed().as_secs_f64() * 1000.0;

            // 2 items per batch, nullifiers and batch id: We allocate a safe capacity.
            let mut public_inputs = Vec::with_capacity(2 * n); 
            for i in 0..n {
                public_inputs.push(batch_ids[i]);
                public_inputs.push(nullifiers[i]);
            }

            // Verification time should not change drastically as it only has to sum
            // the multiplication of the public input with the cosntant verification point.

            let t0 = Instant::now();
            let valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof)
                .expect("verify failed");
            let verify_ms = t0.elapsed().as_secs_f64() * 1000.0;
            assert!(valid, "proof must verify for an honest witness");

            let proof_size_bytes = proof.serialized_size(Compress::Yes);

            writeln!(
                csv,
                "{n},{rep},{setup_ms:.4},{prove_ms:.4},{verify_ms:.4},{num_constraints},{proof_size_bytes}"
            )
            .unwrap();

            println!(
                "batch_size={n:>3} rep={rep} setup={setup_ms:.2}ms prove={prove_ms:.2}ms verify={verify_ms:.2}ms constraints={num_constraints} proof_bytes={proof_size_bytes}"
            );
        }
    }

    println!("\nwrote {out_path}");
}

/* COMPONENT 4 - Wiring all the components togehter: File -> witness -> proof -> verification

- PIPELINE VALIDATION, NOT THE RESEARCH COMPARISON.

- This is a the miniature version: Build the circuit, generate the proof, verify it and time each stage.

- HONEST NOTE: Backend: Groth16 / BLS12-381 — deliberately NOT Artifact A (PLONK/halo2, Pallas-Vesta)
or Artifact B (Nova-nova snark, Pallas-Vesta) since its API is most stable and documented which de-risks
commiting time to two less mature APIs. It is not a data point for Table2 mentioned in the document 
since Groth16 is neither recursive of foldable, so it cannot answer the thesis question. It also 
requires a trusted setup per circuit, which only PLONK requires. Therefore because of these inconsistencies,
this is not a suitable depiction of the broader question. However, it demonstrates feasibility and strong 
methodologies. 

- The next step would be to port this circuit (same constraints, same arithmetizations) onto 
artifact A and artifact B for the head to head comparisons.

*/


use ark_std::env;

use ark_bls12_381::{Bls12_381, Fr as F};
use ark_groth16::Groth16; // The proving mechanism
use ark_snark::SNARK;
use ark_std::rand::rngs::StdRng;
use ark_std::rand::SeedableRng;
use std::time::Instant;

use settlement_zk_prototype::nullifier_circuit::{compute_nullifier_native, NullifierCircuitStruct};
use settlement_zk_prototype::poseidon_initializer::poseidon_test_params;
use settlement_zk_prototype::arithmetize::{u64_to_scalar, bytes_to_scalar};

use settlement_zk_prototype::file_to_scalar::create_raw_csv;



fn main() {
    println!("--- Aritmetize a small number ---");

    let n: F = u64_to_scalar(7);
    println!("u64 7 -> scalar {n}");

    println!("--- Aritmetize bytes ---");

    let h = bytes_to_scalar(b"hello");
    println!("bytes 'hello' -> scalar {h}");

    println!("--- Aritmetize a non-empty CSV file ---");

    let sample_path = env::temp_dir().join("sample_trade.csv");

    create_raw_csv(&sample_path).expect("");

    let bytes = std::fs::read(&sample_path).expect("");
    let secret: F = bytes_to_scalar(&bytes);

    println!("file bytes -> scalar (this is the private witness): {secret}");

    println!("--- Build the public statement ---");

    let params = poseidon_test_params::<F>();
    let batch_id = F::from(20260723u64); // A number that identifies a trade batch
    let nullifier = compute_nullifier_native(&params, secret, batch_id);

    println!("batch_id (public): {batch_id}");
    println!("nullifier = Poseidon(secret, batch_id) (public): {nullifier}");

    println!("--- Trusted setup (circuit shape only, no secret yet) ---");

    let mut rng = StdRng::seed_from_u64(42);

    let setup_circuit = NullifierCircuitStruct{
        params: params.clone(),
        secret: None,
        batch_id,
        nullifier,
    };

    let t0 = Instant::now();

    /* This single function below orchestrates the entire process of transforming the high-level
    Rust constraints into the mathematical objects required to generate Groth16 keys.

    Arithmetization -> QAP(Lagrange Interpolation) -> Evaluation at (tau) -> Preperation for Billinear Pairings.

    The function outputs a Proving key (pk) and a Verification Key (vk).

    The function samples the secret toxic waste parameters (alpha, beta, gamma, delta, tau)) and 
    immediately wipes the raw numbers from memory.

    The pk struct contains the individual wire inputes scaled by a & b, dismantling forging proof elements.
    A_G1 queries, B_G1 queries, B_G2 queries, h_query & l_query containing massive vectors of G1 points 
    so the prover can compute the quotient without knowing tau or delta. One point per polynomial degree.

    So the size of the proving key scales linearly with the number of constraints (gates).

    The vk struct is much smaller, holding curve points ALPHA_G1, BETA_G2, GAMMA_G2, DELTA_G2 and GAMMA_G1. It also 
    contains the points requried for the verifying portion. Only the reference points.

    */

    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit, &mut rng)
    .expect("");
    println!("setup took {:?}", t0.elapsed());

    // 540ms, 640ms, 544ms, 734ms, 578ms (MacBook Air M2 8GB)

    // Slowest, needs to QAP, produce the list of points for each query.

    println!("--- prove (this time with secret plugged in) ---");

    let prove_circuit = NullifierCircuitStruct {
        params: params.clone(),
        secret: Some(secret),
        batch_id,
        nullifier,
    };

    let t0 = Instant::now();
    let proof = Groth16::<Bls12_381>::prove(&pk, prove_circuit, &mut rng).expect("");
    let prove_time = t0.elapsed();

    println!("Tprove = {prove_time:?}   <- this is exactly the metric my project plan uses");

    // 582ms, 559ms, 576ms, 557ms, 577ms (MacBook Air M2 8GB)

    // Medium-slow, takes private inputs and public inputs, combines them into a witness vector
    // and generates the proof.

    println!("--- verify (clearinghouse side - only public inputs, no secret) --- ");
    let public_inputs = [nullifier, batch_id]; // Initialization order in nullifier circuit must match (learned hard way :( )
    let t0 = Instant::now();
    let valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("");

    println!("verify took {:?}, valid = {valid}", t0.elapsed());
    assert!(valid);
    // 159ms, 156ms, 143ms, 162ms, 149ms (MacBook Air M2 8GB)

    println!(" --- sanity check - forged secret must fail to verify --- ");

    let x = bytes_to_scalar(b"gerhardkling");
    let forged_secret: F = secret + x;

    let forged_circuit = NullifierCircuitStruct {
        params,
        secret: Some(forged_secret),
        batch_id,
        nullifier, // claims same public nullifier but using a different secret.
    };

    // Proving now will either fail outright or the nullifier will reject it.

    let cs = ark_relations::gr1cs::ConstraintSystem::<F>::new_ref();
    use ark_relations::gr1cs::ConstraintSynthesizer;

    forged_circuit.generate_constraints(cs.clone()).unwrap();
    println!("forged secret satisfies constraints? {} (must be false)",
        cs.is_satisfied().unwrap()
    );
    // 'false' because the nullifier does not correspond to the forged secret
    
    std::fs::remove_file(&sample_path).ok();
}




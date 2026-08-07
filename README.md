# README made by AI

# Settlement ZK Prototype

This repository contains a Rust-based prototype for a zero-knowledge settlement workflow using Arkworks and Groth16. The project demonstrates a compact end-to-end pipeline for arithmetizing trade-like data, building a nullifier circuit, generating a proof, and verifying it without revealing the underlying secret values.

## Why this project exists

The goal is to show that a settlement-style workflow can be expressed as a zk-SNARK circuit and tested on a laptop. In this prototype, the private witness is a secret value derived from file content, while the public statement is a batch identifier and the resulting nullifier. The system proves that the secret was used correctly without exposing it.

## High-level flow

```text
+-------------------+      +----------------------+      +----------------------+
| Private witness   | ---> | Circuit constraints  | ---> | Groth16 proof        |
| (secret + batch)  |      | (nullifier logic)    |      | (proof + public in) |
+-------------------+      +----------------------+      +----------------------+
                                      |                             |
                                      v                             v
                             +----------------------+      +----------------------+
                             | Public inputs       |      | Verification        |
                             | (batch_id, nullifier)|      | (accept/reject)     |
                             +----------------------+      +----------------------+
```

## How it works

1. A secret value is converted into a scalar field element.
2. A batch identifier is also converted into a field element.
3. A nullifier is computed from the secret and batch identifier.
4. A circuit is built so that the proof attests to the correctness of that relationship.
5. The verifier checks the proof using only the public inputs.

This design is useful for scenarios where a party wants to prove knowledge of a hidden value while revealing only a derived public commitment such as a nullifier.

## Main components

- `src/main.rs` - Demonstrates the full proof lifecycle from witness creation to verification.
- `src/benchmark.rs` - Measures setup, proof generation, and verification time for increasing batch sizes.
- `src/arithmetize.rs` - Converts bytes and integers into field elements.
- `src/nullifier_circuit.rs` - Defines the nullifier circuit used for proof generation.
- `src/batch_circuit.rs` - Extends the idea to multiple entries in a batch.
- `src/poseidon_initializer.rs` - Initializes Poseidon-related parameters for the circuit.
- `src/file_to_scalar.rs` - Converts file data into a scalar witness.

## Benchmarking

The benchmark runner writes CSV results to:

- `benchmark_results.csv`
- `benchmark_summary.csv`

It records metrics such as:

- batch size
- repetition number
- setup time
- prove time
- verify time
- number of constraints
- proof size

## Prerequisites

- Rust and Cargo
- A compatible Rust toolchain for the 2024 edition

## Run the demo

```bash
cargo run --release
```

## Run the benchmark

```bash
cargo run --release --bin benchmark
```

## Notes

This repository is a prototype and research-oriented implementation. It is intended to demonstrate feasibility and provide benchmark data rather than serve as a production-ready settlement system.

## License

This project is provided as-is for experimentation and research purposes.

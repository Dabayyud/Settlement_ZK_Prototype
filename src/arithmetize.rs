
/* COMPONENT 1 - ARITHMETIZATION

Arithmetization involes expressing something as a scalar element finite
field F_r, where r is the *scalar field order* of the curve (for BLS12-381,
r is a ~255-bit prime)

The three common cases: 

1)  A small integer (u64, u32) -> trivially fits in one field element.

2)  A big/arbitrary-length blob (a file, a string) -> too big for one 
    element, or not naturally a number at all -> hash it down to size.

3)  A value you need to inspect *bit-by-bit* inside the circuit (e.g. for a
    range check, "is this trade size < 10,000") -> decompose into bits. 
    This is the case for this project (+ or * natively, not "<").

*/



use ark_bls12_381::Fr as Scalar;
use ark_ff::{BigInteger, PrimeField};
use sha2::{Digest, Sha512};


// CASE 1: 
// U64 -> Scalar

pub fn u64_to_scalar(x: u64) -> Scalar { // cost: 0 constraints. 64-bit -> 256 bit.
    Scalar::from(x)
} 

// CASE 2: 
// Arbitrary bytes -> one field element, via hash-then-reduce. 
// We use SHA-512 as modern CPU's are exceptionally efficient with this hash function.
// `from_le_bytes_mod_order` takes the raw hash digest and reduces it mod r.
// SHA-512 is used as opposed to SHA-256 to reduce modular bias (avoids statistical bias).


pub fn bytes_to_scalar(bytes: &[u8]) -> Scalar {
    // again, SHA-512 is used to minimize modulo bias (between 0 and \(2^{256} \pmod r\))
    // since  BLS12-381 scalar field order (r) is roughly \(2^{255}\).
    // Compute a 512-bit digest (64 bytes)
    let digest = Sha512::digest(bytes);
    Scalar::from_le_bytes_mod_order(&digest) // Reference for speed.
}

// CASE 3:
// This makes range checks and comparisons possible inside of a circuit
// Returns a dynamically allocated vector of booleans. True represents 1 bit and false a 0 bit.

pub fn scalar_to_le_bits(x: &Scalar, num_bits: usize) -> Vec<bool> { 
    let mut bits = x.into_bigint().to_bits_le();
    (& mut bits).truncate(num_bits);
    bits
}

// UNIT TESTING

#[cfg(test)] 

mod tests {
    use super::*;
 
    #[test]
    fn small_int_roundtrip() {
        let s = u64_to_scalar(42);
        assert_eq!(s, Scalar::from(42u64));
    }

    #[test]
    fn bytes_are_deterministic_and_field_valid() {
        let a = bytes_to_scalar(b"trade-batch-0001");
        let b = bytes_to_scalar(b"trade-batch-0001");
        let c = bytes_to_scalar(b"trade-batch-0002");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn bit_decomposition_reconstructs_the_value() {
        let x = Scalar::from(13u64); // 1101
        let bits = scalar_to_le_bits(&x, 8);
        let reconstructed: u64 = bits
            .iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .map(|(i, _)| 1u64 << i)
            .sum();
        assert_eq!(reconstructed, 13);
    }

}


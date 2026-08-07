/* COMPONENT 2 - File -> number (Scalar)

- This is the shape of the operation we would use to serialize arbritrary data into
a private witness 'nullifier secret' that gets fed into component 3.

- 'Path to any file on a disk' -> read bytes -> reduce mod r. The output will always
be a single Scalar no matter how large the file is.

- This component creates a CSV file, hashes its bytes with SHA-512, and reduces the digest mod 
the BLS12-381 scalar field order. This hash is calculated outside of the circuit. It 
is not the hash that will be recomputed inside the circuit (SHA-256 costs thousands of R1CS
constraints per hash because it's built from 32-bit XOR/rotate operations
that don't map cleanly onto field arithmetic). That is why we use Poseidon as it is native to F(p).


*/

use std::io::{self, Write};
use ark_bls12_381::Fr as Scalar;
use ark_ff::PrimeField;
use sha2::{Digest, Sha512};
use std::fs::File;
use std::path::Path;


pub fn files_to_scalar<P>(path: P) -> io::Result<Scalar> 
where
P: AsRef<Path>,{
    let bytes = std::fs::read(path)?;
    let digest = Sha512::digest(&bytes);
    Ok(Scalar::from_le_bytes_mod_order(&digest))
}


pub fn create_raw_csv<P>(path: P) -> io::Result<()> 
where 
P: AsRef<Path>, {

    let mut file = File::create(path)?;

    file.write_all(b"trade_id,symbol,price\n")?;
    file.write_all(b"1,BTC,95000.0\n")?;
    file.write_all(b"2,ETH,3200.0\n")?;

    file.flush()?;
    Ok(()) 
}

#[cfg(test)]
mod tests {

    use super::*; // Imports all modules into current scope

    #[test]
    // Tests creation of the CSV file.

    fn test_csv_creation_and_reading() -> std::io::Result<()> {
        let path = std::env::temp_dir().join("zk_sample_trade_batch.csv");

        create_raw_csv(&path)?;
        assert!(path.exists());

        let metadata = std::fs::metadata(&path)?;
        assert!(metadata.len() > 0);

        std::fs::remove_file(path)?;

        Ok(())
    }

    #[test]    
    // Tests if writing new data changes scalar value.

    fn reads_and_arithmetizes_a_file() {
        let path = std::env::temp_dir().join("zk_sample_trade_batch2.csv");
        
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, "trade_id,asset,qty,price").unwrap();
        writeln!(f, "1,BTC-USD,10,65000").unwrap();
        drop(f);

        let scalar_a = files_to_scalar(&path).unwrap();
        let scalar_b = files_to_scalar(&path).unwrap();

        assert_eq!(scalar_a, scalar_b);

        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "2,ETH-USD,5,3200").unwrap();

        drop(f);
        let scalar_c = files_to_scalar(&path).unwrap();
        assert_ne!(scalar_a, scalar_c);
 
        std::fs::remove_file(&path).ok();
    }

}

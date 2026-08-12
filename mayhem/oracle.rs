// Deterministic oracle for StorageProofChecker — same logic as the fuzz harness, fixed input.
use bp_runtime::{RawStorageProof, StorageProofChecker};
use sp_core::{Blake2Hasher, H256};
use sp_state_machine::{backend::Backend, prove_read, InMemoryBackend};
use std::collections::HashMap;
use std::process::exit;

fn craft_known_storage_proof(
	input_vec: Vec<(Vec<u8>, Vec<u8>)>,
) -> Option<(H256, RawStorageProof)> {
	let storage_proof_vec =
		vec![(None, input_vec.iter().map(|x| (x.0.clone(), Some(x.1.clone()))).collect())];
	let state_version = sp_runtime::StateVersion::default();
	let backend = <InMemoryBackend<Blake2Hasher>>::from((storage_proof_vec, state_version));
	let root = backend.storage_root(std::iter::empty(), state_version).0;
	let vector_element_proof = prove_read(backend, input_vec.iter().map(|x| x.0.as_slice())).ok()?;
	Some((
		root,
		vector_element_proof.iter_nodes().cloned().collect(),
	))
}

fn main() {
	let input = vec![(b"oracle-key".to_vec(), b"oracle-value".to_vec())];
	let unique: Vec<_> = {
		let mut m = HashMap::new();
		for (k, v) in input {
			m.insert(k, v);
		}
		m.into_iter().collect()
	};
	let Some((root, proof)) = craft_known_storage_proof(unique.clone()) else {
		eprintln!("storage proof oracle: FAIL craft");
		exit(1);
	};
	let Ok(mut checker) = StorageProofChecker::<Blake2Hasher>::new(root, proof) else {
		eprintln!("storage proof oracle: FAIL checker");
		exit(1);
	};
	for (key, val) in unique {
		match checker.read_value(&key) {
			Ok(Some(v)) if v == val => {}
			other => {
				eprintln!("storage proof oracle: FAIL read {other:?}");
				exit(1);
			}
		}
	}
	println!("storage proof oracle: OK");
}

// Port of the old honggfuzz harness (fuzz/storage-proof/src/main.rs) to libFuzzer.
// Storage Proof Checker fuzzer — exercises bp_runtime::StorageProofChecker.

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use bp_runtime::{RawStorageProof, StorageProofChecker};
use libfuzzer_sys::fuzz_target;
use sp_core::{Blake2Hasher, H256};
use sp_state_machine::{backend::Backend, prove_read, InMemoryBackend};
use std::collections::HashMap;

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

fn transform_into_unique(input_vec: Vec<(Vec<u8>, Vec<u8>)>) -> Vec<(Vec<u8>, Vec<u8>)> {
	let mut output_hashmap = HashMap::new();
	for key_value_pair in input_vec {
		output_hashmap.insert(key_value_pair.0, key_value_pair.1);
	}
	output_hashmap.into_iter().collect()
}

fn run_once(data: &[u8]) {
	let mut u = Unstructured::new(data);
	let input_vec: Vec<(Vec<u8>, Vec<u8>)> = match Vec::arbitrary(&mut u) {
		Ok(v) => v,
		Err(_) => return,
	};
	if input_vec.is_empty() {
		return;
	}
	let unique_input_vec = transform_into_unique(input_vec);
	let Some((root, proof)) = craft_known_storage_proof(unique_input_vec.clone()) else {
		return;
	};
	let Ok(mut checker) = StorageProofChecker::<Blake2Hasher>::new(root, proof) else {
		return;
	};
	for key_value_pair in unique_input_vec {
		assert_eq!(
			checker.read_value(&key_value_pair.0),
			Ok(Some(key_value_pair.1.clone()))
		);
	}
}

fuzz_target!(|data: &[u8]| {
	let _ = env_logger::try_init();
	run_once(data);
});

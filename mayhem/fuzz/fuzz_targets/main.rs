// Port of the old honggfuzz harness (fuzz/storage-proof/src/main.rs) to libFuzzer.
// Storage Proof Checker fuzzer — exercises bp_runtime::StorageProofChecker.
//
// Faithful to the original: honggfuzz's `fuzz!(|input: T|)` decodes the input with
// `Arbitrary::arbitrary_take_rest`, and every fallible step the original harness
// `.unwrap()`/`.expect()`s is left fallible here — those panics ARE the oracle.

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use sp_core::{Blake2Hasher, H256};
use sp_state_machine::{backend::Backend, prove_read, InMemoryBackend};
use sp_trie::StorageProof;
use std::collections::HashMap;

fn craft_known_storage_proof(input_vec: Vec<(Vec<u8>, Vec<u8>)>) -> (H256, StorageProof) {
	let storage_proof_vec =
		vec![(None, input_vec.iter().map(|x| (x.0.clone(), Some(x.1.clone()))).collect())];
	log::info!("Storage proof vec {:?}", storage_proof_vec);
	let state_version = sp_runtime::StateVersion::default();
	let backend = <InMemoryBackend<Blake2Hasher>>::from((storage_proof_vec, state_version));
	let root = backend.storage_root(std::iter::empty(), state_version).0;
	let vector_element_proof = StorageProof::new(
		prove_read(backend, input_vec.iter().map(|x| x.0.as_slice())).unwrap().iter_nodes(),
	);
	(root, vector_element_proof)
}

fn transform_into_unique(input_vec: Vec<(Vec<u8>, Vec<u8>)>) -> Vec<(Vec<u8>, Vec<u8>)> {
	let mut output_hashmap = HashMap::new();
	let mut output_vec = Vec::new();
	for key_value_pair in input_vec {
		output_hashmap.insert(key_value_pair.0, key_value_pair.1); //Only 1 value per key
	}
	for (key, val) in output_hashmap.iter() {
		output_vec.push((key.clone(), val.clone()));
	}
	output_vec
}

fn run_once(input_vec: Vec<(Vec<u8>, Vec<u8>)>) {
	if input_vec.is_empty() {
		return
	}
	let unique_input_vec = transform_into_unique(input_vec);
	let (root, craft_known_storage_proof) = craft_known_storage_proof(unique_input_vec.clone());
	let checker =
		<bp_runtime::StorageProofChecker<Blake2Hasher>>::new(root, craft_known_storage_proof)
			.expect("Valid proof passed; qed");
	for key_value_pair in unique_input_vec {
		log::info!("Reading value for pair {:?}", key_value_pair);
		assert_eq!(checker.read_value(&key_value_pair.0), Ok(Some(key_value_pair.1.clone())));
	}
}

fuzz_target!(|data: &[u8]| {
	let _ = env_logger::try_init();
	let u = Unstructured::new(data);
	let input_vec: Vec<(Vec<u8>, Vec<u8>)> = match Arbitrary::arbitrary_take_rest(u) {
		Ok(v) => v,
		Err(_) => return,
	};
	run_once(input_vec);
});

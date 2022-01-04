use std::collections::BTreeMap;

use nucypher_core::Address;
use nucypher_core_wasm::*;

use umbral_pre::bindings_wasm::{
    generate_kfrags, reencrypt, PublicKey, SecretKey, Signer, VerifiedCapsuleFrag, VerifiedKeyFrag,
};
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

//
// Test utilities
//

// Downcast a WASM type to a Rust type
// Reference: https://github.com/rustwasm/wasm-bindgen/issues/2231
fn of_js_value_generic<T: FromWasmAbi<Abi = u32>>(
    js: JsValue,
    classname: &str,
) -> Result<T, JsValue> {
    use js_sys::{Object, Reflect};
    let ctor_name = Object::get_prototype_of(&js).constructor().name();
    if ctor_name == classname {
        let ptr = Reflect::get(&js, &JsValue::from_str("ptr"))?;
        let ptr_u32: u32 = ptr.as_f64().ok_or(JsValue::NULL)? as u32;
        let foo = unsafe { T::from_abi(ptr_u32) };
        Ok(foo)
    } else {
        Err(JsValue::NULL)
    }
}

#[wasm_bindgen]
pub fn verified_key_farg_of_js_value(js_value: JsValue) -> Option<VerifiedKeyFrag> {
    of_js_value_generic(js_value, "VerifiedKeyFrag").unwrap_or(None)
}

#[wasm_bindgen]
pub fn node_metadata_of_js_value(js_value: JsValue) -> Option<NodeMetadata> {
    of_js_value_generic(js_value, "NodeMetadata").unwrap_or(None)
}

fn make_hrac() -> HRAC {
    let publisher_verifying_key = SecretKey::random().public_key();
    let bob_verifying_key = SecretKey::random().public_key();
    let label = "Hello, world!".as_bytes();
    HRAC::new(&publisher_verifying_key, &bob_verifying_key, label)
}

fn make_kfrags(delegating_sk: &SecretKey, receiving_sk: &SecretKey) -> Vec<VerifiedKeyFrag> {
    let receiving_pk = receiving_sk.public_key();
    let signer = Signer::new(&delegating_sk);
    let verified_kfrags: Vec<VerifiedKeyFrag> =
        generate_kfrags(&delegating_sk, &receiving_pk, &signer, 2, 3, false, false)
            .iter()
            .map(|kfrag| verified_key_farg_of_js_value(kfrag.clone()).unwrap())
            .collect();
    verified_kfrags
}

fn make_fleet_state_checksum() -> FleetStateChecksum {
    let this_node = Some(make_node_metadata());
    let other_nodes = vec![make_node_metadata(), make_node_metadata()];
    let other_nodes = serde_wasm_bindgen::to_value(&other_nodes).unwrap();
    FleetStateChecksum::new(this_node, other_nodes).unwrap()
}

fn make_node_metadata() -> NodeMetadata {
    let canonical_address = "00000000000000000001".as_bytes();
    let domain = "localhost";
    let timestamp_epoch = 1546300800;
    let verifying_key = SecretKey::random().public_key();
    let encrypting_key = SecretKey::random().public_key();
    let certificate_bytes = "certificate_bytes".as_bytes();
    let host = "https://localhost.com";
    let port = 443;
    let decentralized_identity_evidence = Some(vec![1, 2, 3]);

    let node_metadata_payload = NodeMetadataPayload::new(
        canonical_address,
        domain,
        timestamp_epoch,
        &verifying_key,
        &encrypting_key,
        certificate_bytes,
        host,
        port,
        decentralized_identity_evidence,
    )
    .unwrap();

    let signer = Signer::new(&SecretKey::random());
    NodeMetadata::new(&signer, &node_metadata_payload)
}

//
// MessageKit
//

#[wasm_bindgen_test]
fn message_kit_decrypts() {
    let sk = SecretKey::random();
    let policy_encrypting_key = sk.public_key();
    let plaintext = "Hello, world!".as_bytes();
    let message_kit = MessageKit::new(&policy_encrypting_key, plaintext);

    let decrypted = message_kit.decrypt(&sk).unwrap().to_vec();
    assert_eq!(
        decrypted, plaintext,
        "Decrypted message does not match plaintext"
    );
}

#[wasm_bindgen_test]
fn message_kit_decrypt_reencrypted() {
    // Create a message kit
    let delegating_sk = SecretKey::random();
    let delegating_pk = delegating_sk.public_key();
    let plaintext = "Hello, world!".as_bytes();
    let message_kit = MessageKit::new(&delegating_pk, plaintext);

    // Create key fragments for reencryption
    let receiving_sk = SecretKey::random();
    let receiving_pk = receiving_sk.public_key();
    let verified_kfrags = generate_kfrags(
        &delegating_sk,
        &receiving_pk,
        &Signer::new(&delegating_sk),
        2,
        3,
        false,
        false,
    );

    // Simulate reencryption on the JS side
    let cfrags: Vec<JsValue> = verified_kfrags
        .iter()
        .map(|kfrag| {
            let kfrag = verified_key_farg_of_js_value(kfrag.clone()).unwrap();
            let cfrag = reencrypt(&message_kit.capsule(), &kfrag).to_bytes();
            let cfrag = serde_wasm_bindgen::to_value(&cfrag).unwrap();
            js_sys::Uint8Array::new(&cfrag).into()
        })
        .collect();
    assert_eq!(cfrags.len(), verified_kfrags.len());

    // Decrypt on the Rust side
    let decrypted = message_kit
        .decrypt_reencrypted(&receiving_sk, &delegating_pk, cfrags.into())
        .unwrap();

    assert_eq!(
        &decrypted[..],
        plaintext,
        "Decrypted message does not match plaintext"
    );
}

#[wasm_bindgen_test]
fn message_kit_to_bytes_from_bytes() {
    let sk = SecretKey::random();
    let policy_encrypting_key = sk.public_key();
    let plaintext = "Hello, world!".as_bytes();

    let message_kit = MessageKit::new(&policy_encrypting_key, plaintext);

    assert_eq!(
        message_kit,
        MessageKit::from_bytes(&message_kit.to_bytes()).unwrap(),
        "MessageKit does not roundtrip"
    );
}

//
// HRAC
//

#[wasm_bindgen_test]
fn hrac_to_bytes() {
    let hrac = make_hrac();

    assert_eq!(
        hrac.to_bytes().len(),
        16,
        "HRAC does not serialize to bytes"
    );
}

//
// EncryptedKeyFrag
//

#[wasm_bindgen_test]
fn encrypted_kfrag_decrypt() {
    let hrac = make_hrac();
    let delegating_sk = SecretKey::random();
    let delegating_pk = delegating_sk.public_key();
    let receiving_sk = SecretKey::random();
    let receiving_pk = receiving_sk.public_key();
    let signer = Signer::new(&delegating_sk);

    let verified_kfrags = make_kfrags(&delegating_sk, &receiving_sk);

    let encrypted_kfrag = EncryptedKeyFrag::new(&signer, &receiving_pk, &hrac, &verified_kfrags[0]);

    let decrypted = encrypted_kfrag
        .decrypt(&receiving_sk, &hrac, &delegating_pk)
        .unwrap();
    assert_eq!(
        decrypted.to_bytes(),
        verified_kfrags[0].to_bytes(),
        "Decrypted KFrag does not match"
    );
}

#[wasm_bindgen_test]
fn encrypted_to_bytes_from_bytes() {
    let hrac = make_hrac();
    let delegating_sk = SecretKey::random();
    let receiving_sk = SecretKey::random();
    let receiving_pk = receiving_sk.public_key();
    let signer = Signer::new(&delegating_sk);

    let verified_kfrags = make_kfrags(&delegating_sk, &receiving_sk);
    let encrypted_kfrag = EncryptedKeyFrag::new(&signer, &receiving_pk, &hrac, &verified_kfrags[0]);

    assert_eq!(
        encrypted_kfrag,
        EncryptedKeyFrag::from_bytes(&encrypted_kfrag.to_bytes()).unwrap(),
        "EncryptedKeyFrag does not roundtrip"
    );
}

//
// TreasureMap
//

fn make_assigned_kfrags(
    verified_kfrags: Vec<VerifiedKeyFrag>,
) -> BTreeMap<String, (PublicKey, VerifiedKeyFrag)> {
    verified_kfrags
        .iter()
        .enumerate()
        .map(|(i, vkfrag)| {
            (
                format!("0000000000000000000{}", i + 1).to_string(),
                (SecretKey::random().public_key(), vkfrag.clone()),
            )
        })
        .collect()
}

fn make_treasure_map(publisher_sk: &SecretKey, receiving_sk: &SecretKey) -> TreasureMap {
    let hrac = make_hrac();
    let verified_kfrags = make_kfrags(&publisher_sk, &receiving_sk);

    let assigned_kfrags = make_assigned_kfrags(verified_kfrags);

    TreasureMap::new(
        &Signer::new(&publisher_sk),
        &hrac,
        &SecretKey::random().public_key(),
        &serde_wasm_bindgen::to_value(&assigned_kfrags).unwrap(),
        2,
    )
    .unwrap()
}

#[wasm_bindgen_test]
fn treasure_map_encrypt_decrypt() {
    let publisher_sk = SecretKey::random();
    let receiving_sk = SecretKey::random();

    let treasure_map = make_treasure_map(&publisher_sk, &receiving_sk);

    let publisher_pk = publisher_sk.public_key();
    let recipient_pk = receiving_sk.public_key();
    let signer = Signer::new(&publisher_sk);
    let encrypted = treasure_map.encrypt(&signer, &recipient_pk);

    let decrypted = encrypted.decrypt(&receiving_sk, &publisher_pk).unwrap();

    assert_eq!(
        decrypted, treasure_map,
        "Decrypted TreasureMap does not match"
    );
}

#[wasm_bindgen_test]
fn treasure_map_destinations() {
    let publisher_sk = SecretKey::random();
    let receiving_sk = SecretKey::random();

    let treasure_map = make_treasure_map(&publisher_sk, &receiving_sk);
    let destinations = treasure_map.destinations().unwrap();
    let destinations: Vec<(Address, EncryptedKeyFrag)> =
        serde_wasm_bindgen::from_value(destinations).unwrap();

    assert!(destinations.len() == 3, "Destinations does not match");
    assert_eq!(
        destinations[0].0.as_ref(),
        "00000000000000000001".as_bytes(),
        "Destination does not match"
    );
    assert_eq!(
        destinations[1].0.as_ref(),
        "00000000000000000002".as_bytes(),
        "Destination does not match"
    );
    assert_eq!(
        destinations[2].0.as_ref(),
        "00000000000000000003".as_bytes(),
        "Destination does not match"
    );
}

#[wasm_bindgen_test]
fn encrypted_treasure_map_from_bytes_to_bytes() {
    let publisher_sk = SecretKey::random();
    let receiving_sk = SecretKey::random();
    let treasure_map = make_treasure_map(&publisher_sk, &receiving_sk);

    let encrypted = treasure_map.encrypt(&Signer::new(&publisher_sk), &receiving_sk.public_key());

    assert_eq!(
        encrypted,
        EncryptedTreasureMap::from_bytes(&encrypted.to_bytes()).unwrap(),
        "EncryptedTreasureMap does not roundtrip"
    );
}

//
// ReencryptionRequest
//

#[wasm_bindgen_test]
fn reencryption_request_from_bytes_to_bytes() {
    let publisher_sk = SecretKey::random();
    let policy_encrypting_key = publisher_sk.public_key();
    let plaintext = "Hello, world!".as_bytes();
    let message_kit = MessageKit::new(&policy_encrypting_key, plaintext);
    let capsules = vec![message_kit.capsule()]
        .iter()
        .map(|capsule| JsValue::from_serde(capsule).unwrap())
        .collect();

    let hrac = make_hrac();

    let receiving_sk = SecretKey::random();
    let receiving_pk = receiving_sk.public_key();
    let signer = Signer::new(&publisher_sk);
    let verified_kfrags = make_kfrags(&publisher_sk, &receiving_sk);
    let encrypted_kfrag = EncryptedKeyFrag::new(&signer, &receiving_pk, &hrac, &verified_kfrags[0]);

    let reencryption_request = ReencryptionRequest::new(
        capsules,
        &hrac,
        &encrypted_kfrag,
        &receiving_pk,
        &receiving_pk,
    )
    .unwrap();

    assert_eq!(
        reencryption_request,
        ReencryptionRequest::from_bytes(&reencryption_request.to_bytes()).unwrap(),
        "ReencryptionRequest does not roundtrip"
    )
}

//
// ReencryptionResponse
//

#[wasm_bindgen_test]
fn reencryption_response_verify() {
    // Make capsules
    let alice_sk = SecretKey::random();
    let policy_encrypting_key = alice_sk.public_key(); // TODO: Use a different secret key
    let plaintext = "Hello, world!".as_bytes();
    let message_kit = MessageKit::new(&policy_encrypting_key, plaintext);
    let capsule = message_kit.capsule();
    let capsules = vec![capsule, capsule, capsule];
    let capsules_js: Box<[JsValue]> = capsules
        .iter()
        .map(|capsule| JsValue::from_serde(capsule).unwrap())
        .collect();

    // Make verified key fragments
    let bob_sk = SecretKey::random();
    let kfrags = make_kfrags(&alice_sk, &bob_sk);

    assert_eq!(
        capsules.len(),
        kfrags.len(),
        "Number of Capsules and KFrags does not match"
    );

    // Simulate the reencryption
    let cfrags: Vec<VerifiedCapsuleFrag> = kfrags
        .iter()
        .map(|kfrag| reencrypt(&capsule, kfrag))
        .collect();

    let cfrags_js: Box<[JsValue]> = cfrags
        .iter()
        .map(|kfrag| JsValue::from_serde(kfrag).unwrap())
        .collect();

    let ursula_sk = SecretKey::random();
    let signer = Signer::new(&ursula_sk);
    let reencryption_response =
        ReencryptionResponse::new(&signer, capsules_js.clone(), cfrags_js).unwrap();

    let verified_js = reencryption_response
        .verify(
            capsules_js,
            &alice_sk.public_key(),
            &ursula_sk.public_key(),
            &policy_encrypting_key,
            &bob_sk.public_key(),
        )
        .unwrap();
    let verified: Vec<VerifiedCapsuleFrag> = verified_js
        .iter()
        .map(|vkfrag| vkfrag.into_serde().unwrap())
        .collect();

    assert_eq!(cfrags, verified, "VerifiedCapsuleFrag does not match");

    let as_bytes = reencryption_response.to_bytes();
    assert_eq!(
        as_bytes,
        ReencryptionResponse::from_bytes(&as_bytes)
            .unwrap()
            .to_bytes(),
        "ReencryptionResponse does not roundtrip"
    );
}

//
// RetrievalKit
//

#[wasm_bindgen_test]
fn retrieval_kit() {
    let alice_sk = SecretKey::random();
    let policy_encrypting_key = alice_sk.public_key();
    let plaintext = "Hello, world!".as_bytes();
    let message_kit = MessageKit::new(&policy_encrypting_key, plaintext);

    let retrieval_kit = RetrievalKit::from_message_kit(&message_kit);

    let queried_addresses = retrieval_kit.queried_addresses();
    assert_eq!(
        queried_addresses.len(),
        0,
        "Queried addresses length does not match"
    );

    let as_bytes = retrieval_kit.to_bytes();
    assert_eq!(
        as_bytes,
        RetrievalKit::from_bytes(&as_bytes).unwrap().to_bytes(),
        "RetrievalKit does not roundtrip"
    );
}

//
// RevocationOrder
//

#[wasm_bindgen_test]
fn revocation_order() {
    let delegating_sk = SecretKey::random();
    let receiving_sk = SecretKey::random();
    let verified_kfrags = make_kfrags(&delegating_sk, &receiving_sk);

    let hrac = make_hrac();
    let receiving_pk = receiving_sk.public_key();
    let signer = Signer::new(&delegating_sk);
    let encrypted_kfrag = EncryptedKeyFrag::new(&signer, &receiving_pk, &hrac, &verified_kfrags[0]);

    let ursula_address = "00000000000000000001".as_bytes();
    let revocation_order = RevocationOrder::new(&signer, ursula_address, &encrypted_kfrag).unwrap();

    assert!(revocation_order.verify_signature(&delegating_sk.public_key()));

    let as_bytes = revocation_order.to_bytes();
    assert_eq!(
        as_bytes,
        RevocationOrder::from_bytes(&as_bytes)
            .unwrap()
            .to_bytes()
            .into(),
        "RevocationOrder does not roundtrip"
    );
}

//
// NodeMetadataPayload
//

// See below for the `NodeMetadata` struct.

//
// NodeMetadata
//

#[wasm_bindgen_test]
fn node_metadata() {
    let node_metadata = make_node_metadata();

    let as_bytes = node_metadata.to_bytes();
    assert_eq!(
        as_bytes,
        NodeMetadata::from_bytes(&as_bytes).unwrap().to_bytes(),
        "NodeMetadata does not roundtrip"
    );
}

//
// FleetStateChecksum
//

#[wasm_bindgen_test]
fn fleet_state_checksum_to_bytes() {
    let fleet_state_checksum = make_fleet_state_checksum();

    assert!(
        fleet_state_checksum.to_bytes().len() > 0,
        "FleetStateChecksum does not serialize to bytes"
    );
}

//
// MetadataRequest
//

#[wasm_bindgen_test]
fn metadata_request() {
    let fleet_state_checksum = make_fleet_state_checksum();
    let announce_nodes = vec![make_node_metadata(), make_node_metadata()];
    let announce_nodes_js = serde_wasm_bindgen::to_value(&announce_nodes).unwrap();

    let metadata_request = MetadataRequest::new(&fleet_state_checksum, announce_nodes_js).unwrap();

    let nodes_js = metadata_request.announce_nodes();
    let nodes: Vec<NodeMetadata> = nodes_js
        .iter()
        .cloned()
        .map(|js_node| node_metadata_of_js_value(js_node).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(nodes, announce_nodes);

    let as_bytes = metadata_request.to_bytes();
    assert_eq!(
        as_bytes,
        MetadataRequest::from_bytes(&as_bytes).unwrap().to_bytes(),
        "MetadataRequest does not roundtrip"
    );
}

//
// VerifiedMetadataResponse
//

#[wasm_bindgen_test]
fn metadata_response_payload() {
    let announce_nodes = vec![make_node_metadata(), make_node_metadata()];
    let timestamp_epoch = 1546300800;

    let metadata_response_payload = MetadataResponsePayload::new(
        timestamp_epoch,
        serde_wasm_bindgen::to_value(&announce_nodes).unwrap(),
    );

    let nodes_js = metadata_response_payload.announce_nodes();
    let nodes: Vec<NodeMetadata> = nodes_js
        .iter()
        .cloned()
        .map(|js_node| node_metadata_of_js_value(js_node).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(nodes, announce_nodes, "Announce nodes does not match");
}

//
// MetadataResponse
//

#[wasm_bindgen_test]
fn metadata_response() {
    let announce_nodes = vec![make_node_metadata(), make_node_metadata()];
    let timestamp_epoch = 1546300800;
    let metadata_response_payload = MetadataResponsePayload::new(
        timestamp_epoch,
        serde_wasm_bindgen::to_value(&announce_nodes).unwrap(),
    );
    let signer = Signer::new(&SecretKey::random());

    let metadata_response = MetadataResponse::new(&signer, &metadata_response_payload);

    let as_bytes = metadata_response.to_bytes();
    assert_eq!(
        as_bytes,
        MetadataResponse::from_bytes(&as_bytes).unwrap().to_bytes(),
        "MetadataResponse does not roundtrip"
    );
}

#![no_std]

// Use `wee_alloc` as the global allocator.
extern crate wee_alloc;
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern crate alloc;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;
use js_sys::{Error, Uint8Array};
use nucypher_core::ProtocolObject;
use umbral_pre::bindings_wasm::{
    Capsule, PublicKey, SecretKey, Signer, VerifiedCapsuleFrag, VerifiedKeyFrag,
};
use wasm_bindgen::{
    prelude::{wasm_bindgen, JsValue},
    JsCast,
};

mod utils;

fn map_js_err<T: fmt::Display>(err: T) -> JsValue {
    Error::new(&format!("{}", err)).into()
}

trait AsBackend<T> {
    fn as_backend(&self) -> &T;
}

trait FromBackend<T> {
    fn from_backend(backend: T) -> Self;
}

fn to_bytes<'a, T, U>(obj: &T) -> Box<[u8]>
where
    T: AsBackend<U>,
    U: ProtocolObject<'a>,
{
    obj.as_backend().to_bytes()
}

fn from_bytes<'a, T, U>(data: &'a [u8]) -> Result<T, JsValue>
where
    T: FromBackend<U>,
    U: ProtocolObject<'a>,
{
    U::from_bytes(data).map(T::from_backend).map_err(map_js_err)
}

fn try_make_address(address_bytes: &[u8]) -> Result<nucypher_core::Address, JsValue> {
    let addr = nucypher_core::Address::from_slice(address_bytes)
        .ok_or_else(|| Error::new(&format!("Invalid address: {:?}", address_bytes)))?;
    Ok(addr)
}

//
// MessageKit
//

fn js_value_to_u8_vec(array_of_uint8_arrays: &[JsValue]) -> Result<Vec<Vec<u8>>, JsValue> {
    let vec_vec_u8 = array_of_uint8_arrays
        .iter()
        .filter_map(|u8_array| {
            u8_array
                .dyn_ref::<Uint8Array>()
                .map(|u8_array| u8_array.to_vec())
        })
        .collect::<Vec<_>>();

    if vec_vec_u8.len() != array_of_uint8_arrays.len() {
        Err("Invalid Array of Uint8Arrays.".to_string().into()).into()
    } else {
        Ok(vec_vec_u8)
    }
}

#[wasm_bindgen]
#[derive(PartialEq, Debug)]
pub struct MessageKit {
    backend: nucypher_core::MessageKit,
}

impl AsBackend<nucypher_core::MessageKit> for MessageKit {
    fn as_backend(&self) -> &nucypher_core::MessageKit {
        &self.backend
    }
}

impl FromBackend<nucypher_core::MessageKit> for MessageKit {
    fn from_backend(backend: nucypher_core::MessageKit) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl MessageKit {
    #[wasm_bindgen(constructor)]
    pub fn new(policy_encrypting_key: &PublicKey, plaintext: &[u8]) -> MessageKit {
        Self {
            backend: nucypher_core::MessageKit::new(policy_encrypting_key.inner(), plaintext),
        }
    }

    pub fn decrypt(&self, sk: &SecretKey) -> Result<Box<[u8]>, JsValue> {
        self.backend.decrypt(sk.inner()).map_err(map_js_err)
    }

    #[wasm_bindgen(js_name = decryptReencrypted)]
    pub fn decrypt_reencrypted(
        &self,
        sk: &SecretKey,
        policy_encrypting_key: &PublicKey,
        cfrags: Box<[JsValue]>,
    ) -> Result<Box<[u8]>, JsValue> {
        utils::set_panic_hook();

        let backend_cfrags: Vec<umbral_pre::VerifiedCapsuleFrag> = js_value_to_u8_vec(&cfrags)?
            .iter()
            .cloned()
            .map(|bytes| {
                VerifiedCapsuleFrag::from_verified_bytes(&bytes)
                    .expect("Failed to deserialize VerifiedCapsuleFrag")
                    .inner()
            })
            .collect();

        self.backend
            .decrypt_reencrypted(sk.inner(), policy_encrypting_key.inner(), &backend_cfrags)
            .map_err(map_js_err)
    }

    #[wasm_bindgen(method, getter)]
    pub fn capsule(&self) -> Capsule {
        Capsule::new(self.backend.capsule)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<MessageKit, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// HRAC
//

#[wasm_bindgen]
#[derive(PartialEq)]
pub struct HRAC {
    backend: nucypher_core::HRAC,
}

impl AsBackend<nucypher_core::HRAC> for HRAC {
    fn as_backend(&self) -> &nucypher_core::HRAC {
        &self.backend
    }
}

impl FromBackend<nucypher_core::HRAC> for HRAC {
    fn from_backend(backend: nucypher_core::HRAC) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl HRAC {
    #[wasm_bindgen(constructor)]
    pub fn new(
        publisher_verifying_key: &PublicKey,
        bob_verifying_key: &PublicKey,
        label: &[u8],
    ) -> HRAC {
        Self {
            backend: nucypher_core::HRAC::new(
                publisher_verifying_key.inner(),
                bob_verifying_key.inner(),
                label,
            ),
        }
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        self.backend.as_ref().to_vec().into_boxed_slice()
    }
}

impl HRAC {
    pub fn inner(&self) -> nucypher_core::HRAC {
        self.backend
    }
}

//
// EncryptedKeyFrag
//

#[wasm_bindgen]
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct EncryptedKeyFrag {
    backend: nucypher_core::EncryptedKeyFrag,
}

impl AsBackend<nucypher_core::EncryptedKeyFrag> for EncryptedKeyFrag {
    fn as_backend(&self) -> &nucypher_core::EncryptedKeyFrag {
        &self.backend
    }
}

impl FromBackend<nucypher_core::EncryptedKeyFrag> for EncryptedKeyFrag {
    fn from_backend(backend: nucypher_core::EncryptedKeyFrag) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl EncryptedKeyFrag {
    #[wasm_bindgen(constructor)]
    pub fn new(
        signer: &Signer,
        recipient_key: &PublicKey,
        hrac: &HRAC,
        verified_kfrag: &VerifiedKeyFrag,
    ) -> EncryptedKeyFrag {
        Self {
            backend: nucypher_core::EncryptedKeyFrag::new(
                signer.inner(),
                recipient_key.inner(),
                &hrac.backend,
                verified_kfrag.inner(),
            ),
        }
    }

    pub fn decrypt(
        &self,
        sk: &SecretKey,
        hrac: &HRAC,
        publisher_verifying_key: &PublicKey,
    ) -> Result<VerifiedKeyFrag, JsValue> {
        self.backend
            .decrypt(sk.inner(), &hrac.inner(), publisher_verifying_key.inner())
            .map_err(map_js_err)
            .map(VerifiedKeyFrag::new)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<EncryptedKeyFrag, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

impl EncryptedKeyFrag {
    pub fn inner(&self) -> nucypher_core::EncryptedKeyFrag {
        self.backend.clone()
    }
}

//
// TreasureMap
//

#[wasm_bindgen]
#[derive(Clone, PartialEq, Debug)]
pub struct TreasureMap {
    backend: nucypher_core::TreasureMap,
}

impl AsBackend<nucypher_core::TreasureMap> for TreasureMap {
    fn as_backend(&self) -> &nucypher_core::TreasureMap {
        &self.backend
    }
}

impl FromBackend<nucypher_core::TreasureMap> for TreasureMap {
    fn from_backend(backend: nucypher_core::TreasureMap) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl TreasureMap {
    #[wasm_bindgen(constructor)]
    pub fn new(
        signer: &Signer,
        hrac: &HRAC,
        policy_encrypting_key: &PublicKey,
        assigned_kfrags: &JsValue,
        threshold: u8,
    ) -> Result<TreasureMap, JsValue> {
        // Using String here to avoid issue where Deserialize is not implemented
        // for every possible lifetime.
        let assigned_kfrags: BTreeMap<String, (PublicKey, VerifiedKeyFrag)> =
            serde_wasm_bindgen::from_value(assigned_kfrags.clone())?;
        let assigned_kfrags_backend = assigned_kfrags
            .iter()
            .map(|(address, (key, vkfrag))| {
                (
                    try_make_address(address.as_bytes()).unwrap(),
                    (key.inner(), vkfrag.inner()),
                )
            })
            .collect::<Vec<_>>();
        Ok(Self {
            backend: nucypher_core::TreasureMap::new(
                signer.inner(),
                &hrac.backend,
                policy_encrypting_key.inner(),
                assigned_kfrags_backend,
                threshold,
            ),
        })
    }

    pub fn encrypt(&self, signer: &Signer, recipient_key: &PublicKey) -> EncryptedTreasureMap {
        EncryptedTreasureMap {
            backend: self.backend.encrypt(signer.inner(), recipient_key.inner()),
        }
    }

    #[wasm_bindgen(method, getter)]
    pub fn destinations(&self) -> Result<JsValue, JsValue> {
        let mut result = Vec::new();
        for (address, ekfrag) in &self.backend.destinations {
            result.push((
                address,
                EncryptedKeyFrag {
                    backend: ekfrag.clone(),
                },
            ));
        }
        Ok(serde_wasm_bindgen::to_value(&result)?)
    }

    #[wasm_bindgen(method, getter)]
    pub fn hrac(&self) -> HRAC {
        HRAC {
            backend: self.backend.hrac,
        }
    }

    #[wasm_bindgen(method, getter)]
    pub fn threshold(&self) -> u8 {
        self.backend.threshold
    }

    #[wasm_bindgen(method, getter, js_name = policyEncryptingKey)]
    pub fn policy_encrypting_key(&self) -> PublicKey {
        PublicKey::new(self.backend.policy_encrypting_key)
    }

    #[wasm_bindgen(method, getter, js_name = publisherVerifyingKey)]
    pub fn publisher_verifying_key(&self) -> PublicKey {
        PublicKey::new(self.backend.publisher_verifying_key)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<TreasureMap, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// EncryptedTreasureMap
//

#[wasm_bindgen]
#[derive(PartialEq, Debug)]
pub struct EncryptedTreasureMap {
    backend: nucypher_core::EncryptedTreasureMap,
}

impl AsBackend<nucypher_core::EncryptedTreasureMap> for EncryptedTreasureMap {
    fn as_backend(&self) -> &nucypher_core::EncryptedTreasureMap {
        &self.backend
    }
}

impl FromBackend<nucypher_core::EncryptedTreasureMap> for EncryptedTreasureMap {
    fn from_backend(backend: nucypher_core::EncryptedTreasureMap) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl EncryptedTreasureMap {
    pub fn decrypt(
        &self,
        sk: &SecretKey,
        publisher_verifying_key: &PublicKey,
    ) -> Result<TreasureMap, JsValue> {
        self.backend
            .decrypt(sk.inner(), publisher_verifying_key.inner())
            .map_err(map_js_err)
            .map(|treasure_map| TreasureMap {
                backend: treasure_map,
            })
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<EncryptedTreasureMap, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// ReencryptionRequest
//

#[wasm_bindgen]
#[derive(PartialEq, Debug)]
pub struct ReencryptionRequest {
    backend: nucypher_core::ReencryptionRequest,
}

impl AsBackend<nucypher_core::ReencryptionRequest> for ReencryptionRequest {
    fn as_backend(&self) -> &nucypher_core::ReencryptionRequest {
        &self.backend
    }
}

impl FromBackend<nucypher_core::ReencryptionRequest> for ReencryptionRequest {
    fn from_backend(backend: nucypher_core::ReencryptionRequest) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl ReencryptionRequest {
    #[wasm_bindgen(constructor)]
    pub fn new(
        capsules: Box<[JsValue]>,
        hrac: &HRAC,
        encrypted_kfrag: &EncryptedKeyFrag,
        publisher_verifying_key: &PublicKey,
        bob_verifying_key: &PublicKey,
    ) -> Result<ReencryptionRequest, JsValue> {
        utils::set_panic_hook();

        let capsules_backend: Vec<umbral_pre::Capsule> = js_value_to_u8_vec(&capsules)?
            .iter()
            .map(|capsule| *Capsule::from_bytes(capsule).unwrap().inner())
            .collect();

        Ok(Self {
            backend: nucypher_core::ReencryptionRequest::new(
                &capsules_backend,
                &hrac.backend,
                &encrypted_kfrag.backend,
                &publisher_verifying_key.inner(),
                &bob_verifying_key.inner(),
            ),
        })
    }

    #[wasm_bindgen(method, getter)]
    pub fn hrac(&self) -> HRAC {
        HRAC {
            backend: self.backend.hrac,
        }
    }

    #[wasm_bindgen(method, getter)]
    pub fn publisher_verifying_key(&self) -> PublicKey {
        PublicKey::new(self.backend.publisher_verifying_key)
    }

    #[wasm_bindgen(method, getter)]
    pub fn bob_verifying_key(&self) -> PublicKey {
        PublicKey::new(self.backend.bob_verifying_key)
    }

    #[wasm_bindgen(method, getter)]
    pub fn encrypted_kfrag(&self) -> EncryptedKeyFrag {
        EncryptedKeyFrag {
            backend: self.backend.encrypted_kfrag.clone(),
        }
    }

    #[wasm_bindgen(method, getter)]
    pub fn capsules(&self) -> Vec<JsValue> {
        self.backend
            .capsules
            .iter()
            .map(|capsule| Capsule::new(*capsule))
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<ReencryptionRequest, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// ReencryptionResponse
//

#[wasm_bindgen]
pub struct ReencryptionResponse {
    backend: nucypher_core::ReencryptionResponse,
}

impl AsBackend<nucypher_core::ReencryptionResponse> for ReencryptionResponse {
    fn as_backend(&self) -> &nucypher_core::ReencryptionResponse {
        &self.backend
    }
}

impl FromBackend<nucypher_core::ReencryptionResponse> for ReencryptionResponse {
    fn from_backend(backend: nucypher_core::ReencryptionResponse) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl ReencryptionResponse {
    #[wasm_bindgen(constructor)]
    pub fn new(
        signer: &Signer,
        capsules: Box<[JsValue]>,
        verified_capsule_frags: Box<[JsValue]>,
    ) -> Result<ReencryptionResponse, JsValue> {
        let capsules_backend: Vec<umbral_pre::Capsule> = js_value_to_u8_vec(&capsules)?
            .iter()
            .map(|capsule| *Capsule::from_bytes(capsule).unwrap().inner())
            .collect();

        let vcfrags_backend: Vec<umbral_pre::VerifiedCapsuleFrag> =
            js_value_to_u8_vec(&verified_capsule_frags)?
                .iter()
                .map(|vcfrag| {
                    VerifiedCapsuleFrag::from_verified_bytes(vcfrag)
                        .unwrap()
                        .inner()
                })
                .collect();

        Ok(ReencryptionResponse {
            backend: nucypher_core::ReencryptionResponse::new(
                signer.inner(),
                &capsules_backend,
                &vcfrags_backend,
            ),
        })
    }

    pub fn verify(
        &self,
        capsules: Box<[JsValue]>,
        alice_verifying_key: &PublicKey,
        ursula_verifying_key: &PublicKey,
        policy_encrypting_key: &PublicKey,
        bob_encrypting_key: &PublicKey,
    ) -> Result<Box<[JsValue]>, JsValue> {
        let capsules: Vec<Capsule> = capsules
            .iter()
            .map(|capsule| JsValue::into_serde(capsule).unwrap())
            .collect();
        let capsules_backend = capsules
            .iter()
            .map(|capsule| *capsule.inner())
            .collect::<Vec<_>>();
        let vcfrags_backend = self
            .backend
            .verify(
                &capsules_backend,
                alice_verifying_key.inner(),
                ursula_verifying_key.inner(),
                policy_encrypting_key.inner(),
                bob_encrypting_key.inner(),
            )
            .unwrap();

        let vcfrags_backend_js = vcfrags_backend
            .iter()
            .map(|vcfrag| VerifiedCapsuleFrag::new(vcfrag.clone()))
            .map(|vcfrag| JsValue::from_serde(&vcfrag).unwrap())
            .collect();
        Ok(vcfrags_backend_js)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<ReencryptionResponse, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// RetrievalKit
//

#[wasm_bindgen]
pub struct RetrievalKit {
    backend: nucypher_core::RetrievalKit,
}

impl AsBackend<nucypher_core::RetrievalKit> for RetrievalKit {
    fn as_backend(&self) -> &nucypher_core::RetrievalKit {
        &self.backend
    }
}

impl FromBackend<nucypher_core::RetrievalKit> for RetrievalKit {
    fn from_backend(backend: nucypher_core::RetrievalKit) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl RetrievalKit {
    #[wasm_bindgen(js_name = fromMessageKit)]
    pub fn from_message_kit(message_kit: &MessageKit) -> Self {
        Self {
            backend: nucypher_core::RetrievalKit::from_message_kit(&message_kit.backend),
        }
    }

    #[wasm_bindgen(constructor)]
    pub fn new(capsule: &Capsule, queried_addresses: JsValue) -> Result<RetrievalKit, JsValue> {
        // Using String here to avoid issue where Deserialize is not implemented
        // for every possible lifetime.
        let queried_addresses: Vec<String> = serde_wasm_bindgen::from_value(queried_addresses)?;
        let addresses_backend = queried_addresses
            .iter()
            .map(|address| try_make_address(address.as_bytes()).unwrap())
            .collect::<Vec<_>>();
        Ok(Self {
            backend: nucypher_core::RetrievalKit::new(capsule.inner(), addresses_backend),
        })
    }

    #[wasm_bindgen(method, getter)]
    pub fn capsule(&self) -> Capsule {
        Capsule::new(self.backend.capsule)
    }

    #[wasm_bindgen(method, getter)]
    pub fn queried_addresses(&self) -> Vec<JsValue> {
        self.backend
            .queried_addresses
            .iter()
            .map(|address| JsValue::from_serde(&address).unwrap())
            .collect::<Vec<_>>()
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<RetrievalKit, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// RevocationOrder
//

#[wasm_bindgen]
pub struct RevocationOrder {
    backend: nucypher_core::RevocationOrder,
}

impl AsBackend<nucypher_core::RevocationOrder> for RevocationOrder {
    fn as_backend(&self) -> &nucypher_core::RevocationOrder {
        &self.backend
    }
}

impl FromBackend<nucypher_core::RevocationOrder> for RevocationOrder {
    fn from_backend(backend: nucypher_core::RevocationOrder) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl RevocationOrder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        signer: &Signer,
        ursula_address: &[u8],
        encrypted_kfrag: &EncryptedKeyFrag,
    ) -> Result<RevocationOrder, JsValue> {
        let address = try_make_address(ursula_address)?;
        Ok(Self {
            backend: nucypher_core::RevocationOrder::new(
                signer.inner(),
                &address,
                &encrypted_kfrag.backend,
            ),
        })
    }

    #[wasm_bindgen]
    pub fn verify_signature(&self, alice_verifying_key: &PublicKey) -> bool {
        self.backend.verify_signature(alice_verifying_key.inner())
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<RevocationOrder, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// NodeMetadataPayload
//

// TODO: Find a way to avoid this conversion?
pub fn from_canonical(data: &ethereum_types::H160) -> &str {
    core::str::from_utf8(&data[..]).unwrap()
}

#[wasm_bindgen]
pub struct NodeMetadataPayload {
    backend: nucypher_core::NodeMetadataPayload,
}

#[wasm_bindgen]
impl NodeMetadataPayload {
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        canonical_address: &[u8],
        domain: &str,
        timestamp_epoch: u32,
        verifying_key: &PublicKey,
        encrypting_key: &PublicKey,
        certificate_bytes: &[u8],
        host: &str,
        port: u16,
        decentralized_identity_evidence: Option<Vec<u8>>,
    ) -> Result<NodeMetadataPayload, JsValue> {
        let address = try_make_address(canonical_address)?;
        Ok(Self {
            backend: nucypher_core::NodeMetadataPayload {
                canonical_address: address,
                domain: domain.to_string(),
                timestamp_epoch,
                verifying_key: *verifying_key.inner(),
                encrypting_key: *encrypting_key.inner(),
                certificate_bytes: certificate_bytes.into(),
                host: host.to_string(),
                port,
                decentralized_identity_evidence: decentralized_identity_evidence
                    .map(|v| v.into_boxed_slice()),
            },
        })
    }

    #[wasm_bindgen(method, getter)]
    pub fn canonical_address(&self) -> Vec<u8> {
        self.backend.canonical_address.as_ref().to_vec()
    }

    #[wasm_bindgen(method, getter)]
    pub fn verifying_key(&self) -> PublicKey {
        PublicKey::new(self.backend.verifying_key)
    }

    #[wasm_bindgen(method, getter)]
    pub fn encrypting_key(&self) -> PublicKey {
        PublicKey::new(self.backend.encrypting_key)
    }

    #[wasm_bindgen(method, getter)]
    pub fn decentralized_identity_evidence(&self) -> Option<Box<[u8]>> {
        self.backend.decentralized_identity_evidence.clone()
    }

    #[wasm_bindgen(method, getter)]
    pub fn domain(&self) -> String {
        self.backend.domain.clone()
    }

    #[wasm_bindgen(method, getter)]
    pub fn host(&self) -> String {
        self.backend.host.clone()
    }

    #[wasm_bindgen(method, getter)]
    pub fn port(&self) -> u16 {
        self.backend.port
    }

    #[wasm_bindgen(method, getter)]
    pub fn timestamp_epoch(&self) -> u32 {
        self.backend.timestamp_epoch
    }

    #[wasm_bindgen(method, getter)]
    pub fn certificate_bytes(&self) -> Box<[u8]> {
        self.backend.certificate_bytes.clone()
    }
}

//
// NodeMetadata
//

#[wasm_bindgen(method, getter)]
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct NodeMetadata {
    backend: nucypher_core::NodeMetadata,
}

impl AsBackend<nucypher_core::NodeMetadata> for NodeMetadata {
    fn as_backend(&self) -> &nucypher_core::NodeMetadata {
        &self.backend
    }
}

impl FromBackend<nucypher_core::NodeMetadata> for NodeMetadata {
    fn from_backend(backend: nucypher_core::NodeMetadata) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl NodeMetadata {
    #[wasm_bindgen(constructor)]
    pub fn new(signer: &Signer, payload: &NodeMetadataPayload) -> Self {
        Self {
            backend: nucypher_core::NodeMetadata::new(signer.inner(), &payload.backend),
        }
    }

    pub fn verify(&self) -> bool {
        self.backend.verify()
    }

    #[wasm_bindgen(method, getter)]
    pub fn payload(&self) -> NodeMetadataPayload {
        NodeMetadataPayload {
            backend: self.backend.payload.clone(),
        }
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<NodeMetadata, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// FleetStateChecksum
//

#[wasm_bindgen]
pub struct FleetStateChecksum {
    backend: nucypher_core::FleetStateChecksum,
}

impl AsBackend<nucypher_core::FleetStateChecksum> for FleetStateChecksum {
    fn as_backend(&self) -> &nucypher_core::FleetStateChecksum {
        &self.backend
    }
}

impl FromBackend<nucypher_core::FleetStateChecksum> for FleetStateChecksum {
    fn from_backend(backend: nucypher_core::FleetStateChecksum) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl FleetStateChecksum {
    #[wasm_bindgen(constructor)]
    pub fn new(
        // TODO: Fix lack of reference leading to accidental freeing of memory
        //       https://github.com/rustwasm/wasm-bindgen/issues/2370
        // this_node: Option<&NodeMetadata>,
        this_node: Option<NodeMetadata>,
        other_nodes: JsValue,
    ) -> Result<FleetStateChecksum, JsValue> {
        let other_nodes: Vec<NodeMetadata> = serde_wasm_bindgen::from_value(other_nodes)?;
        let other_nodes_backend = other_nodes
            .iter()
            .map(|node| node.backend.clone())
            .collect::<Vec<_>>();
        Ok(Self {
            backend: nucypher_core::FleetStateChecksum::from_nodes(
                this_node.map(|node| node.backend).as_ref(),
                &other_nodes_backend,
            ),
        })
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        self.backend.as_ref().to_vec().into_boxed_slice()
    }
}

//
// MetadataRequest
//

#[wasm_bindgen]
pub struct MetadataRequest {
    backend: nucypher_core::MetadataRequest,
}

impl AsBackend<nucypher_core::MetadataRequest> for MetadataRequest {
    fn as_backend(&self) -> &nucypher_core::MetadataRequest {
        &self.backend
    }
}

impl FromBackend<nucypher_core::MetadataRequest> for MetadataRequest {
    fn from_backend(backend: nucypher_core::MetadataRequest) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl MetadataRequest {
    #[wasm_bindgen(constructor)]
    pub fn new(
        fleet_state_checksum: &FleetStateChecksum,
        announce_nodes: JsValue,
    ) -> Result<MetadataRequest, JsValue> {
        let announce_nodes: Vec<NodeMetadata> = serde_wasm_bindgen::from_value(announce_nodes)?;
        let nodes_backend = announce_nodes
            .iter()
            .map(|node| node.backend.clone())
            .collect::<Vec<_>>();
        Ok(Self {
            backend: nucypher_core::MetadataRequest::new(
                &fleet_state_checksum.backend,
                &nodes_backend,
            ),
        })
    }

    #[wasm_bindgen(method, getter, js_name = fleetStateChecksum)]
    pub fn fleet_state_checksum(&self) -> FleetStateChecksum {
        FleetStateChecksum {
            backend: self.backend.fleet_state_checksum,
        }
    }

    #[wasm_bindgen(method, getter, js_name = announceNodes)]
    pub fn announce_nodes(&self) -> Vec<JsValue> {
        self.backend
            .announce_nodes
            .iter()
            .map(|node| NodeMetadata {
                backend: node.clone(),
            })
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<MetadataRequest, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

//
// MetadataResponsePayload
//

#[wasm_bindgen]
pub struct MetadataResponsePayload {
    backend: nucypher_core::MetadataResponsePayload,
}

#[wasm_bindgen]
impl MetadataResponsePayload {
    #[wasm_bindgen(constructor)]
    pub fn new(timestamp_epoch: u32, announce_nodes: JsValue) -> Self {
        let announce_nodes: Vec<NodeMetadata> =
            serde_wasm_bindgen::from_value(announce_nodes).unwrap();
        let nodes_backend = announce_nodes
            .iter()
            .map(|node| node.backend.clone())
            .collect::<Vec<_>>();
        MetadataResponsePayload {
            backend: nucypher_core::MetadataResponsePayload::new(timestamp_epoch, &nodes_backend),
        }
    }

    #[wasm_bindgen(method, getter)]
    pub fn timestamp_epoch(&self) -> u32 {
        self.backend.timestamp_epoch
    }

    #[wasm_bindgen(method, getter)]
    pub fn announce_nodes(&self) -> Vec<JsValue> {
        self.backend
            .announce_nodes
            .iter()
            .map(|node| NodeMetadata {
                backend: node.clone(),
            })
            .map(JsValue::from)
            .collect()
    }
}

//
// MetadataResponse
//

#[wasm_bindgen]
pub struct MetadataResponse {
    backend: nucypher_core::MetadataResponse,
}

impl AsBackend<nucypher_core::MetadataResponse> for MetadataResponse {
    fn as_backend(&self) -> &nucypher_core::MetadataResponse {
        &self.backend
    }
}

impl FromBackend<nucypher_core::MetadataResponse> for MetadataResponse {
    fn from_backend(backend: nucypher_core::MetadataResponse) -> Self {
        Self { backend }
    }
}

#[wasm_bindgen]
impl MetadataResponse {
    #[wasm_bindgen(constructor)]
    pub fn new(signer: &Signer, response: &MetadataResponsePayload) -> Self {
        Self {
            backend: nucypher_core::MetadataResponse::new(signer.inner(), &response.backend),
        }
    }

    pub fn verify(&self, verifying_pk: &PublicKey) -> Result<MetadataResponsePayload, JsValue> {
        self.backend
            .verify(verifying_pk.inner())
            .ok_or("Invalid signature")
            .map_err(map_js_err)
            .map(|backend| MetadataResponsePayload { backend })
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<MetadataResponse, JsValue> {
        from_bytes(data)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Box<[u8]> {
        to_bytes(self)
    }
}

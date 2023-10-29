use lazy_static::lazy_static;

use openmls::{
	prelude::*,
	credentials::{CredentialBundle, CredentialType},
};

lazy_static! {
	// define a static MLS group configuration use for all groups
	
	// In future, multiple configurations could be made and 
	// profiled for difference performance characteristics,
	// then allow the user to dynamically set their performance based on risk tolerance

	static ref MLS_GROUP_CONFIG_DEFAULT: MlsGroupConfig = MlsGroupConfig::builder()
		.wire_format_policy(PURE_CIPHERTEXT_WIRE_FORMAT_POLICY)
		.padding_size(16)
		.use_ratchet_tree_extension(true)
		.sender_ratchet_configuration(SenderRatchetConfiguration::new(
			20,   // out_of_order_tolerance
			1000, // maximum_forward_distance
		))
		.build();
}

//
// identification functions //
//

fn new_mls_credential(
	identity: Vec<u8>,
	credential_type: CredentialType,
	signature_algorithm: SignatureScheme,
	backend: &impl OpenMlsCryptoProvider,
) -> Result<Credential, CredentialError> {

	let credential_bundle = CredentialBundle::new(identity, credential_type, signature_algorithm, backend)?;
	
	let credential_id = credential_bundle
		.credential()
		.signature_key()
		.tls_serialize_detached()
		.expect("Signature key should serialise");
	
	// store the new credential bundle in the backend's keystore
	backend
		.key_store()
		.store(&credential_id, &credential_bundle)
		.expect("Backend accepts new stored keys");

	
	Ok(credential_bundle.into_parts().0)
}

pub fn new_key_package(
	credential: &Credential,
	backend: &impl OpenMlsCryptoProvider,
) -> Result<KeyPackage, KeyPackageBundleNewError> {

	// Fetch an existing credential bundle from the key store
	let credential_id = credential
		.signature_key()
		.tls_serialize_detached()
		.expect("Credential should serialise");

	let credential_bundle = backend
		.key_store()
		.read(&credential_id)
		.expect("Keystore should return bundle handle");

	// Create the key package bundle
	let key_package_bundle = KeyPackageBundle::new(
		&[Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519],
		&credential_bundle,
		backend,
		vec![],
	)
	.expect("Should generate a new keypack");

	// Hash the keypack to make an ID for it
	let key_package_id = key_package_bundle
		.key_package()
		.hash_ref(backend.crypto())
		.expect("Should hash the new keypack");
	
	// Store it in the key store
	backend
		.key_store()
		.store(key_package_id.value(), &key_package_bundle)
		.expect("Backend should accept the new keypack");

	Ok(key_package_bundle.into_parts().0)
}

//
// group functions //
//

pub fn new_mls_group_from_welcome(backend: &impl OpenMlsCryptoProvider, welcome: Welcome) -> Result<MlsGroup, WelcomeError> {

	MlsGroup::new_from_welcome(
		backend,
		&MLS_GROUP_CONFIG_DEFAULT,
		welcome,
		None,
	)

}

pub fn new_mls_credential_from_identity(identity: Vec<u8>,backend: &impl OpenMlsCryptoProvider) -> Result<Credential, CredentialError> {

	new_mls_credential(
		identity,
		CredentialType::Basic,
		SignatureScheme::ED25519,
		backend,
	)

}

pub fn new_mls_group(backend: &impl OpenMlsCryptoProvider,key_package: KeyPackage) -> MlsGroup {

	let group_id = GroupId::from_slice(b"Placeholder_Group_ID");

	MlsGroup::new(
		backend,
		&MLS_GROUP_CONFIG_DEFAULT,
		group_id,
		key_package
			.hash_ref(backend.crypto())
			.expect("Keypack should hash")
			.as_slice(),
	)
	.expect("MLS group should be created")

}
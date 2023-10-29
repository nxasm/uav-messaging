use libp2p::{identity::Keypair, PeerId};
use openmls::{
	group::MlsGroup,
	prelude::{KeyPackage, MlsMessageOut, ProcessedMessage, Welcome, WelcomeError, ParseMessageError},
};
use openmls_rust_crypto::OpenMlsRustCrypto;

use log::{debug};

use crate::crypto::*;

struct Identity {
	network_key: Keypair,
	mls_keypack: KeyPackage,
	peer_id: PeerId,
}

pub struct Node {
	backend: OpenMlsRustCrypto,
	mls_group: Option<MlsGroup>,
	identity: Identity,
	is_group_leader: bool,
}

impl Default for Node {
	fn default() -> Node {

		let network_key = Keypair::generate_ed25519();
		let peer_id = PeerId::from_public_key(&network_key.public());
		let backend = OpenMlsRustCrypto::default();
		let credential = new_mls_credential_from_identity(peer_id.into(), &backend)
			.expect("Should generate a new credential");
		let key_package = new_key_package(&credential, &backend).unwrap();

		Node {
			backend,
			mls_group: None,
			is_group_leader: false,
			identity: Identity {
				network_key: network_key,
				mls_keypack: key_package,
				peer_id: peer_id,
			},
		}

	}
}

impl Node {
	pub fn create_group(&mut self) {
		self.mls_group = Some(new_mls_group(
			&self.backend,
			self.identity.mls_keypack.clone(),
		));
		self.is_group_leader = true;
	}

	pub fn add_node_to_group(&mut self, key_package: KeyPackage) -> (MlsMessageOut, Welcome) {
		let group = self.mls_group.as_mut()
			.expect("Should have a group");
		
		let (m_out, welcome) = group
			.add_members(&self.backend, &[key_package])
			.expect("Should add a new member");
		
		group
			.merge_pending_commit()
			.expect("Should merge pending commit");
		
		(m_out, welcome)
	}

	pub fn join_group(&mut self, welcome: Welcome) -> Result<(), WelcomeError> {
		self.mls_group = Some(new_mls_group_from_welcome(&self.backend, welcome)?);
		self.is_group_leader = false;
		Ok(())
	}

	pub fn create_message(&mut self, msg: &str) -> Result<MlsMessageOut, ()> {
		Ok(
			self.mls_group
				.as_mut()
				.expect("Should have a group")
				.create_message(&self.backend, msg.as_bytes())
				.expect("Should create an application message")
		)
	}

	pub fn parse_message(&mut self, msg_out: MlsMessageOut) -> Result<Option<String>, ParseMessageError> {
		if self.mls_group.is_none() {
			return Ok(None);
		}
		let unverified_message = self.mls_group
			.as_mut()
			.expect("Node should have a group")
			.parse_message(msg_out.into(), &self.backend)?;
		
		let processed_message = self.mls_group
			.as_mut()
			.expect("Node should have a group")
			.process_unverified_message(
				unverified_message,
				None,
				&self.backend,
			)
			.expect("Should be able to verify the parsed message");
		
		match processed_message {
			ProcessedMessage::ApplicationMessage(application_message) => {
				debug!("Processed application message: {:?}", application_message);
				// Check the message
				return Ok(Some(
					String::from_utf8(application_message.into_bytes())
						.expect("Should parse message")
				));
			}
			ProcessedMessage::StagedCommitMessage(staged_commit) => {
				debug!("Processed staged commit: {:?}", staged_commit);
				self.mls_group
					.as_mut()
					.expect("group")
					.merge_staged_commit(*staged_commit)
					.expect("Could not merge Commit.");
				Ok(None)
			}

			ProcessedMessage::ProposalMessage(_) => {
				debug!("Proposal message unimplemented: {:?}", processed_message);
				Ok(None)
			}
		}
	}

	pub fn is_group_leader(&self) -> bool {
		self.is_group_leader
	}

	pub fn has_group(&self) -> bool {
		self.mls_group.is_some()
	}
	
	pub fn get_key_package(&self) -> KeyPackage {
		self.identity.mls_keypack.clone()
	}
	
	pub fn get_network_keypair(&self) -> Keypair {
		self.identity.network_key.clone()
	}

	pub fn get_peer_id(&self) -> PeerId {
		self.identity.peer_id.clone()
	}
	
}

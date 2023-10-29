use futures::lock::Mutex;
use futures::StreamExt;
use libp2p::{
  floodsub::{self, FloodsubEvent},
  mdns,
  swarm::SwarmEvent,
  PeerId, 
  Swarm,
};
use openmls::prelude::{
  KeyPackage, MlsMessageOut, TlsDeserializeTrait, TlsSerializeTrait, Welcome,
};

use std::sync::Arc;
use async_std::channel;
use log::{info, debug};
use colored::Colorize;

use crate::Node;
use super::{
	MlsChatBehaviour,
	NetworkOutput
};

pub type MsgReceiver = channel::Receiver<(PeerId, Vec<u8>)>;
pub type MsgSender = channel::Sender<(PeerId, Vec<u8>)>;
pub type NetworkSender = channel::Sender<Vec<u8>>;
pub type NetworkReceiver = channel::Receiver<Vec<u8>>;

/// The network_handler function is an asynchronous function intended to be run as a spawned task.
///
/// It takes in a Swarm object with MlsChatBehaviour, a NetworkReceiver, and a MsgSender.
///
/// This function is responsible for setting up and managing a distributed, peer-to-peer network node in a chat application. It sets up a new topic in the Floodsub network (which allows messages to be published to multiple subscribers) and manages different types of events in the network, including new connections, disconnections, and receiving messages.
///
/// # Arguments
///
/// * swarm - A mutable Swarm object with MlsChatBehaviour. This object represents a P2P network node.
/// * receiver - A NetworkReceiver object that is used to receive messages from other parts of the application.
/// * sender - A MsgSender object that is used to send messages to other parts of the application.
///
/// # Behavior
///
/// The function subscribes to the floodsub topic "airspaceA" and then enters a loop where it waits for either network events or messages from the application.
///
/// When a network event occurs, the function handles the event based on its type. For example, it logs new connections and disconnections, adds newly discovered peers to the floodsub view, and removes expired peers from the view. If a message is received that is part of the "airspaceA" topic, it sends the message's source and data to other parts of the application using the MsgSender.
///
/// When a message from the application is received via the NetworkReceiver, the function publishes this message to the "airspaceA" floodsub topic.
///
/// # Panics
///
/// The function will panic if sending a message via the MsgSender fails. This is most likely to occur if the receiver has been dropped.
///
/// # Examples
///
/// This function is typically used as a part of a larger chat application and would be spawned as a task alongside other concurrent tasks:
/// ```rust
/// async_std::task::spawn( network_handler(swarm, receiver, sender) ;
/// ```
/// # Note
/// 
/// This function runs indefinitely. To stop it, you would need to break the loop, typically by dropping the sender of the NetworkReceiver or MsgSender, causing the .select_next_some() to return None.
pub async fn network_handler(
  mut swarm: Swarm<MlsChatBehaviour>,
  net_task_receiver: NetworkReceiver,
  msg_task_sender: MsgSender,
) {
  // Create a Floodsub topic
  let chat = floodsub::Topic::new("airspaceA");
  
  swarm.behaviour_mut().floodsub.subscribe(chat.clone());
  
  let mut receiver = net_task_receiver.fuse();
  
  loop {
    futures::select! {
      event = swarm.select_next_some() => {
        match event {
          SwarmEvent::NewListenAddr { address, .. } => {
            info!("Listening on {}", address);
          }
          SwarmEvent::ConnectionEstablished { peer_id, endpoint,.. } => {
            debug!("Connected to {} on {}", peer_id, endpoint.get_remote_address());
          }
          SwarmEvent::ConnectionClosed { peer_id,.. } => {
            debug!("Disconnected from {}", peer_id);
          }
          SwarmEvent::Behaviour(NetworkOutput::Mdns(mdns::Event::Discovered(list))) => {
            for (peer_id, _multiaddr) in list {
              info!("mDNS discovered a new peer: {peer_id}");
              swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
            }
          }
          SwarmEvent::Behaviour(NetworkOutput::Mdns(mdns::Event::Expired(list))) => {
            for (peer, _multiaddr) in list {
              debug!("mDNS expired: {:?}", peer);
              if !swarm.behaviour_mut().mdns.has_node(&peer) {
                swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer);
              }
            }
          },
          SwarmEvent::Behaviour(NetworkOutput::Floodsub(FloodsubEvent::Message(message))) if message.topics.contains(&chat) => {
            msg_task_sender.send((message.source, message.data)).await.unwrap();
          },
          _ => {} // ignore all other events
        }
      },
      message = receiver.select_next_some() => {
        swarm.behaviour_mut().floodsub.publish(chat.clone(), message);
      }
    }
  }
}

/// Asynchronous function handling received messages within a network.
///
/// This function operates as an ongoing task responsible for processing messages received
/// from the `msg_receiver` within a peer-to-peer network. The messages are processed based on their content, 
/// with three primary cases covered: handling key packages, MLS outgoing messages (MlsMessageOut), 
/// and welcome messages.
///
/// # Arguments
///
/// * `network_task_sender`: A `NetworkSender` that sends processed messages to other parts of the application or network.
/// * `msg_receiver`: A `MsgReceiver` used to receive messages from the network or other parts of the application.
/// * `node`: A shared, mutable reference to the `Node` object which represents the current node in the network.
///
/// # Behavior
///
/// The function runs indefinitely, processing messages as they are received. 
///
/// Upon receiving a message, it tries to convert the message into a `KeyPackage`. If successful, 
/// it checks if the node is a group leader and, if so, adds the member associated with the key 
/// package to the group and sends a welcome message and a join message for existing members.
///
/// If the message cannot be converted into a `KeyPackage`, the function attempts to convert it 
/// into a `MlsMessageOut`. If successful, it tries to parse the message and print it.
///
/// If the message cannot be converted into either a `KeyPackage` or `MlsMessageOut`, 
/// the function tries to deserialize it into a `Welcome` message and have the node join an existing group.
///
/// If all conversions and deserializations fail, it simply prints the message and the sender's information.
///
/// # Panics
///
/// The function will panic if sending a message via the `network_task_sender` fails.
///
/// # Example
///
/// Typically, the function would be run as a task along with other concurrent tasks:
/// 
/// ```rust
/// async_std::task::spawn(
///     message_handler(network_task_sender, msg_receiver, node);
/// );
/// ```
///
/// # Note
///
/// To stop the function, you'd typically need to break the loop, most likely by dropping the `MsgReceiver` or `NetworkSender`,
/// causing `.select_next_some()` to return `None`.
///
pub async fn message_handler(
	network_task_sender: NetworkSender,
	msg_task_receiver: MsgReceiver,
	node: Arc<Mutex<Node>>,
) {
  
  let mut msg_receiver = msg_task_receiver.fuse();
  
  loop {
    let (peer, message) = msg_receiver.select_next_some().await;
    let mut node_ref = node.lock().await;
    let bytes_array: &[u8] = &message;
    
		if let Ok(key_package) = KeyPackage::try_from(bytes_array) {
			if node_ref.is_group_leader() { // can perform any authentication check here

				let (msg_out, welcome) = node_ref.add_node_to_group(key_package);
				let welcome_serialized = welcome.tls_serialize_detached().unwrap();
				let msg_out_serialized = msg_out.tls_serialize_detached().unwrap();

				network_task_sender.send(welcome_serialized).await.unwrap();
				network_task_sender.send(msg_out_serialized).await.unwrap();

				println!("Added {:?} to the group",peer);
			}
		} 
    
		else if let Ok(msg_out) = MlsMessageOut::try_from_bytes(bytes_array) {
			match node_ref.parse_message(msg_out) {
				Ok(msg) => {
					if let Some(str_msg) = msg {
						println!("{}: {}", peer.to_string().red(), str_msg.blue());
					}
				}
				Err(_) => {
					println!("Received unknown message");
				}
			}
		} 
    
		else if let Ok(welcome) = Welcome::tls_deserialize(&mut &*bytes_array) {
			if let Ok(()) = node_ref.join_group(welcome) {
				println!("Received welcome from {:?}", peer);
			} else {
				println!("Failed to join group");
			}
		} 
		
		else {
			println!("Received: '{:?}' from {:?}", message, peer);
		}
	}
  
}
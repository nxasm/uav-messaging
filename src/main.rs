use futures::lock::Mutex;
use futures::StreamExt;
use libp2p::{
  floodsub::Floodsub,
  mdns,
  swarm::SwarmBuilder,
};

use std::sync::Arc;
use std::error::Error;
use async_std::{prelude::*, channel, io};
use log::{error};

mod network;
mod node;
mod commands;
mod crypto;

use crate::node::Node;
use crate::commands::parse_cmd;
use crate::network::{
  transport::build_tcp_transport,
  MlsChatBehaviour,
  tasks::{
    network_handler,
    message_handler
  },
};

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
  env_logger::init();
  
  // // commented out for file logging. Uncomment to enable logging to the file "nodes.log"
  // match simple_logging::log_to_file("nodes.log", LevelFilter::Info) {
  //   Ok(_) => {}
  //   Err(e) => {
  //     println!("Could not log to file: {}", e)
  //   }
  // }

  let node = Arc::new(Mutex::new( Node::default() ));
  let node_ref = node.lock().await;
  
  
  let network_key = node_ref.get_network_keypair();
  let peer_id = node_ref.get_peer_id();
  drop (node_ref); // release the lock
  
  let transport = build_tcp_transport(&network_key).await?;
  
  // Create a Swarm to manage peers and events
  let mut swarm = SwarmBuilder::with_async_std_executor(
    transport,
    MlsChatBehaviour {
      floodsub: Floodsub::new(peer_id),
      mdns: mdns::async_io::Behaviour::new(mdns::Config::default(), peer_id)?,
    },
    peer_id,
  )
  .build();

  swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

  // create communication channels for communication between the two asynchronous tasks
  let (net_task_sender, net_task_receiver) = channel::unbounded();
  let (msg_task_sender, msg_task_receiver) = channel::unbounded();

  // This is the first async task: the network event loop, which handles the events triggered by the network behaviours
  async_std::task::spawn(network_handler(swarm, net_task_receiver, msg_task_sender));

  // this second asynchronous task handles message opertaions - it parses the events handled by the network task as they happen
  async_std::task::spawn(message_handler(net_task_sender.clone(), msg_task_receiver, node.clone()));

  // SETUP COMPLETE //

  println!("Welcome. Type 'help' for a list of commands.");

  // we now begin processing the stdin, which 
  let mut stdin = io::BufReader::new(io::stdin()).lines();
  
  while let Some(Ok(line)) = stdin.next().await {
    let node_ref = &mut node.lock().await;
    match parse_cmd(node_ref, line) {

      Ok(msg) => {
        if msg.is_empty() {
          continue;
        }
        net_task_sender.send(msg).await.unwrap();
      }

      Err(_) => {
        error!("Error parsing stdin");
      }

    }
  }
  
  Ok(())
}

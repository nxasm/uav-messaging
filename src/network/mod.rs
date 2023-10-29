use libp2p::{
  floodsub::{Floodsub, FloodsubEvent},
  mdns,
  swarm::{NetworkBehaviour},
};

pub mod tasks;
pub mod transport;

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false, out_event = "NetworkOutput")]
pub struct MlsChatBehaviour {
  pub floodsub: Floodsub,
  pub mdns: mdns::async_io::Behaviour,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum NetworkOutput {
  Floodsub(FloodsubEvent),
  Mdns(mdns::Event),
}

impl From<FloodsubEvent> for NetworkOutput {
  fn from(event: FloodsubEvent) -> NetworkOutput {
    NetworkOutput::Floodsub(event)
  }
}

impl From<mdns::Event> for NetworkOutput {
  fn from(event: mdns::Event) -> NetworkOutput {
    NetworkOutput::Mdns(event)
  }
}
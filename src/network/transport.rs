use libp2p::{
  core,
  tcp,
  dns,
  websocket,
  yamux,
  noise,
	Transport,
	PeerId,
};

use std::error::Error;

pub async fn build_tcp_transport(key: &libp2p::identity::Keypair) -> Result<core::transport::Boxed<(PeerId, core::muxing::StreamMuxerBox)>, Box<dyn Error>> {

	let tcp_conf = tcp::Config::new()
		.listen_backlog(1024)
		.nodelay(true);

	let dns_tcp = dns::DnsConfig::system(tcp::async_io::Transport::new( tcp_conf.clone() )).await?;
	let dns_websocket = websocket::WsConfig::new(
		dns::DnsConfig::system(tcp::async_io::Transport::new( tcp_conf.clone() )).await?
	);

	let transport = dns_tcp
		.or_transport(dns_websocket)
		.upgrade(core::upgrade::Version::V1)
		.authenticate(noise::Config::new(key).unwrap())
		.multiplex(yamux::Config::default())
		.timeout(std::time::Duration::from_secs(20))
		.boxed();

	return Ok(transport);
}
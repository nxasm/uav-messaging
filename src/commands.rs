use colored::Colorize;
use openmls::prelude::TlsSerializeTrait;
use clearscreen;

use crate::node::Node;

type Message = Vec<u8>;

static HELP_TEXT: &str = "\n Usage:
	create            create a new group
	join              join an existing group
	send <message>    send a message to the group

	clear             clear the screen
	exit              exit the program
	help              display this help text
\n";

// Command line helper for Node actions
pub fn parse_cmd(node: &mut Node, line: String) -> Result<Message, ()> {
  let input = line.split_whitespace();
	
	let mut msg = Vec::new();
	for cmd in input.clone() {

		match cmd {
			"create" => {
				println!("Creating new group ... ");
				node.create_group();
			}

			"join" => {
				println!("Sending keys ... ");

				msg = node
					.get_key_package()
					.tls_serialize_detached()
					.expect("key should serialize");
			}

			"send" => {
				if node.has_group() == false {
					println!("You must create or join a group before sending a message");
					break;
				}

				let user_msg = input.clone().skip(1).collect::<Vec<&str>>().join(" ");
				msg = node
					.create_message(user_msg.as_str())
					.expect("message should be signed using group credentials")
					.tls_serialize_detached()
					.expect("message should serialize");

				print!("\x1B[F\x1B[2K"); // move up a line and clear it

				println!("{}: {}", "me".to_string().red(), user_msg);
				break;
			}

			"clear" => {
				match clearscreen::clear() {
					Ok(_) => {}
					Err(e) => {
						println!("Could not clear screen: {}", e);
					}
				}
				break;
			}

			"exit" => {
				println!( "{}", "Exiting ...".to_string().red() );
				// Any actions that need to happen when a node severs communication intentionally go here
				std::process::exit(0);
			}

			"help" => {
				println!( "{}", HELP_TEXT );
			}

			_ => {
				println!("Unknown command: {}\nRun 'help' to see the list of commands", cmd);
				break;
			}
		}

	}

  Ok(msg)
}

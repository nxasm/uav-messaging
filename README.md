# How to run
Start with `cargo run`.

Can use `RUST_LOG` environment variable to show extra logs, `info` and `debug` are the two useful levels:
`RUST_LOG=debug cargo run`

# Usage
Once the program has started, you may enter commands into std-input.

There are three primary actions:
1. create a group
2. join a group that you have discovered
3. send a message to the group you are in

```
Usage:
	create            create a new group
	join              join an existing group
	send <message>    send a message to the group

	clear             clear the screen
	exit              exit the program
	help              display this help text
```

To perform a demonstration;
1. Open a terminal, launch the program, and do command: `create`
2. Open another terminal, launch the program
3. Observe mDNS discovery, both instances should report "new peer discovered"
4. On the second terminal, do command: `join`
5. Observe keys are transferred, group is updated to add new member
6. On either terminal, do `send <your_message>` to test sending your message
7. Add extra terminals if desired
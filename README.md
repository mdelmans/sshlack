# Welcome to sshLack

SSHLack is an ssh chatroom written in Rust.

## How to run?

1. Clone the repo

```
$ git clone ...
```

2. Generate a new SSH key pair

```
$ ssh-keygen -t ed25519 -f sshlack_key
```
3. Start the server

```
$ RUST_LOG=info cargo run
```

4. Login

```
$ ssh {username}@127.0.0.1 -p 2222
```

On first login, type in a password you want to use for your username.

5. Have fun!
<img src="https://github.com/mdelmans/sshlack/blob/main/logo.png?raw=true" alt="Logo" width="100"/>

# Welcome to sshLack

SSHLack is an ssh chatroom written in Rust.

Login to a public chat

```sh
$ ssh {your_username}@sshlack.com 
```
On first login type in a password you want to use for your account.

<img src="https://github.com/mdelmans/sshlack/blob/main/screenshot.png?raw=true" alt="Logo" width="500"/>

## How to start your own server?

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

On first login, type in a password you want to use for your account.

5. Have fun!

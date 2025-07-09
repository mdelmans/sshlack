use sshlack::app_server::AppServer;

use env_logger;
use log::{error, info};

use clap::Parser;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

use russh::keys::PrivateKey;

/// SSHLack server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// IP to listen on
    #[arg(short, long, default_value_t = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))]
    address: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 2222)]
    port: u16,

    /// Certificate file path
    #[arg(short, long, default_value = "sshlack_key")]
    cert_path: PathBuf,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    match PrivateKey::read_openssh_file(args.cert_path.as_path()) {
        Ok(pem) => {
            info!("Starting sshlack server on {}:{}", args.address, args.port);
            AppServer::run(args.address, args.port, pem)
                .await
                .expect("Failed running server");
        }
        Err(e) => {
            error!("Error loading certificate: {}", e);
            std::process::exit(1);
        }
    }
}

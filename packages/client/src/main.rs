use std::io::Error;
use std::process::exit;
use tonic;
use clap::Parser;
use ble_over_ip_proto::ble_proxy_client::BleProxyClient;
use ble_over_ip_proto::DiscoverRequest;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct ServerArgs {
    #[arg(short, long)]
    url: String,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("BLE Over IP Client");
    println!("Connecting...");

    let args = ServerArgs::parse();

    let connection_result = BleProxyClient::connect(args.url).await;

    if (connection_result.is_err()) {
        eprintln!("Could not connect: {}", connection_result.err().unwrap());
        exit(1);
        return Ok(())
    }

    let mut client = connection_result.unwrap();

    println!("Connected successfully. Starting discovery...");

    let discovery_request = tonic::Request::new(DiscoverRequest {});
    let discovery_response = client.discover_device(discovery_request).await?;

    let services = discovery_response.into_inner().services;
    println!("Services length: {}", services.len());
    for s in services {
        println!("├─ Service {}", s.uuid);
        for c in s.characteristics {
            println!("├── Characteristic {}", c.uuid);
        }
    }

    println!("Discovery finished!");

    Ok(())
}
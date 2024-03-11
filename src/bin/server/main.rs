use std::time::Duration;
use btleplug::api::{bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::{Parser};
use tokio;
use std::error::Error;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct ServerArgs {
    #[arg(short, long)]
    device_name: String,

    #[arg(short, long, default_value_t = 5555)]
    listen_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = ServerArgs::parse();

    println!("Hello from server!");
    println!("Checking adapters...");


    let bleManager = Manager::new().await.unwrap();
    let adapters = bleManager.adapters().await?;
    let adapter = adapters.into_iter().nth(0).unwrap();

    println!("Starting scan...");
    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    for p in adapter.peripherals().await.unwrap() {
        let properties = p.properties().await.unwrap().unwrap();
        println!("Device found: {} at {}", properties.local_name.unwrap(), properties.address.to_string())
    }

    println!("Fin!");
    Ok(())
}

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


    let ble_manager = Manager::new().await.unwrap();
    let adapters = ble_manager.adapters().await?;
    let adapter = adapters.into_iter().nth(0).unwrap();

    println!("Starting scan...");
    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    for p in adapter.peripherals().await.unwrap() {
        let properties = p.properties().await.unwrap().unwrap();
        let dev_name = properties.local_name.unwrap();
        if (dev_name.eq(&args.device_name)) {
            let dev_address = properties.address;
            println!("Device found: {} at {}", dev_name, dev_address.to_string());
            p.connect().await?;
            p.discover_services().await?;
            for s in p.services() {
                println!("├─ Service {}", s.uuid.to_string());
                for c in s.characteristics {
                    println!("├── Characteristic {}", c.uuid);
                }
            }
            break;
        }
    }

    println!("Fin!");
    Ok(())
}

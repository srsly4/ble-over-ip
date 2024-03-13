use std::time::Duration;
use btleplug::api::{bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::{Parser};
use tokio;
use tokio_stream::{Stream};
use std::error::Error;
use std::pin::Pin;
use tonic::{Request, Response, Status, transport::Server};

use ble_over_ip_proto::ble_proxy_server::BleProxy;
use ble_over_ip_proto::{DeviceDescription, DiscoverRequest, ReadRequest, ReadResponse, SubscribeEvent, SubscribeRequest, WriteRequest, WriteResponse};

pub struct BleProxyImpl {}

#[tonic::async_trait]
impl BleProxy for BleProxyImpl {
    async fn discover_device(&self, request: Request<DiscoverRequest>) -> Result<Response<DeviceDescription>, Status> {
        todo!()
    }

    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<ReadResponse>, Status> {
        todo!()
    }

    async fn write(&self, request: Request<WriteRequest>) -> Result<Response<WriteResponse>, Status> {
        todo!()
    }

    type SubscribeStream = Pin<Box<dyn Stream<Item = Result<SubscribeEvent, Status>> + Send>>;

    async fn subscribe(&self, request: Request<SubscribeRequest>) -> Result<Response<Self::SubscribeStream>, Status> {
        todo!()
    }
}

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
    tokio::time::sleep(Duration::from_secs(5)).await;

    for p in adapter.peripherals().await.unwrap() {
        let properties = p.properties().await.unwrap().unwrap();
        if properties.local_name.is_none() {
            continue;
        }
        let dev_name = properties.local_name.unwrap();
        println!("Checking device: {}", dev_name);
        if (dev_name.eq(&args.device_name)) {
            let dev_address = properties.address;
            println!("Device found: {} at {}", dev_name, dev_address.to_string());
            create_server(p).await?;
            break;
        }
    }

    println!("Fin!");
    Ok(())
}

async fn create_server(device: Peripheral) -> Result<(), Box<dyn Error>> {
    device.connect().await?;
    device.discover_services().await?;
    for s in device.services() {
        println!("├─ Service {}", s.uuid.to_string());
        for c in s.characteristics {
            println!("├── Characteristic {}", c.uuid);
            for d in c.descriptors {
                println!("├─── Descriptor {}", d.uuid);
            }
        }
    }
    device.disconnect().await?;
    Ok(())
}
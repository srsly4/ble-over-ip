use std::collections::BTreeSet;
use std::time::Duration;
use btleplug::api::{bleuuid::uuid_from_u16, Central, CharPropFlags, Manager as _, Peripheral as _, PeripheralProperties, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::{Parser};
use tokio;
use tokio_stream::{Stream};
use std::error::Error;
use std::ops::Deref;
use std::pin::Pin;
use tonic::{Request, Response, Status, transport::Server};

use ble_over_ip_proto::ble_proxy_server::{BleProxy, BleProxyServer};
use ble_over_ip_proto::{Characteristic, ConnectRequest, ConnectResponse, DeviceDescription, DiscoverRequest, ReadRequest, ReadResponse, Service, SubscribeEvent, SubscribeRequest, WriteRequest, WriteResponse};

pub struct BleProxyImpl {
    services: BTreeSet<btleplug::api::Service>,
    properties: PeripheralProperties,
    peripheral: Peripheral,
}

#[tonic::async_trait]
impl BleProxy for BleProxyImpl {
    async fn discover_device(&self, _request: Request<DiscoverRequest>) -> Result<Response<DeviceDescription>, Status> {
        println!("Discover device request!");
        let mut services: Vec<Service> = vec![];


        println!("Services count: {}", self.services.len());
        for s in &self.services {
            println!("├─ Service {}", s.uuid.to_string());
            let mut characteristics: Vec<Characteristic> = vec![];
            for c in &s.characteristics {
                println!("├── Characteristic {}", c.uuid);
                characteristics.push(Characteristic {
                    uuid: c.uuid.to_string(),
                    can_read: c.properties.contains(CharPropFlags::READ),
                    can_write: c.properties.contains(CharPropFlags::WRITE) || c.properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE),
                    can_subscribe: c.properties.contains(CharPropFlags::NOTIFY) || c.properties.contains(CharPropFlags::INDICATE),
                });
            }
            services.push(Service {
                uuid: s.uuid.to_string(),
                characteristics,
            })
        }


        Ok(Response::new(DeviceDescription {
            services,
            uuid: self.properties.address.to_string(),
            name: self.properties.local_name.clone().unwrap_or("unknown".to_string()),
        }))
    }

    async fn connect_to_device(&self, _request: Request<ConnectRequest>) -> Result<Response<ConnectResponse>, Status> {
        let result = self.peripheral.connect().await;

        if (result.is_err()) {
            return Ok(Response::new(ConnectResponse {
                is_ok: false,
                error: Some(result.err().unwrap().to_string()),
            }))
        }

        Ok (Response::new(ConnectResponse {
            is_ok: true,
            error: None,
        }))
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
            create_server(p, args.listen_port).await?;
            break;
        }
    }

    println!("Fin!");
    Ok(())
}

async fn create_server(device: Peripheral, port: u16) -> Result<(), Box<dyn Error>> {
    device.connect().await?;
    device.discover_services().await?;

    let services = device.services();
    for s in device.services() {
        println!("├─ Service {}", s.uuid.to_string());
        for c in s.characteristics {
            println!("├── Characteristic {}", c.uuid);
        }
    }

    let properties = device.properties().await?.unwrap();

    device.disconnect().await?;
    let ble_proxy = BleProxyImpl {
        properties,
        peripheral: device,
        services,
    };

    let addr = format!("0.0.0.0:{}", port.to_string()).parse()?;

    println!("Starting server at port {}", port.to_string());

    Server::builder()
        .add_service(BleProxyServer::new(ble_proxy))
        .serve(addr)
        .await?;

    Ok(())
}
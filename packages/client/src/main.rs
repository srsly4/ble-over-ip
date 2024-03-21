use std::collections::HashSet;
use std::process::exit;

use bluster::gatt::characteristic::{Characteristic, Properties};
use bluster::gatt::characteristic;
use bluster::gatt::event::Event;
use bluster::gatt::service::Service;
use bluster::Peripheral;
use clap::Parser;
use futures_channel::mpsc::channel;
use tonic;
use tonic::codegen::tokio_stream::StreamExt;
use uuid::Uuid;

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

    if connection_result.is_err() {
        eprintln!("Could not connect: {}", connection_result.err().unwrap());
        exit(1);
    }

    let mut client = connection_result.unwrap();

    println!("Connected successfully. Starting discovery...");

    let discovery_request = tonic::Request::new(DiscoverRequest {});
    let discovery_response = client.discover_device(discovery_request).await?;
    let discovery = discovery_response.into_inner();


    println!("Starting peripheral device...");

    let peripheral_result = Peripheral::new().await;
    if peripheral_result.is_err() {
        eprintln!("Could not create peripheral device: {}", peripheral_result.err().unwrap());
        exit(1);
    }
    let peripheral = peripheral_result.unwrap();

    let (sender, receiver) = channel(1);
    for s in &discovery.services {
        println!("├─ Service {}", s.uuid);
        let mut characteristics: HashSet<Characteristic> = HashSet::new();
        for c in &s.characteristics {
            println!("├── Characteristic {}", c.uuid);
            characteristics.insert(Characteristic::new(
                c.uuid.parse().unwrap(),
                Properties::new(
                    if c.can_read {
                        Some(
                        characteristic::Read(
                            characteristic::Secure::Insecure(sender.clone()))
                    )
                    } else { None },
                    if c.can_write {
                        Some(
                            characteristic::Write::WithoutResponse(
                                sender.clone()
                            )
                        )
                    } else { None },
                    if c.can_subscribe {
                        Some(
                            sender.clone()
                        )
                    } else { None },
                    if c.can_subscribe {
                        Some(sender.clone())
                    } else { None },
                ),
                None,
                HashSet::new(),
            ));
        }
        let p_service = Service::new(
            s.uuid.parse().unwrap(),
            true,
            characteristics,
        );
        peripheral.add_service(&p_service).unwrap();
    }

    let characteristic_handler = async {
        let mut rx = receiver;

        while let Some(event) = rx.next().await {
            // process request from client device
            match event {
                Event::ReadRequest(read_request) => {

                }
                Event::WriteRequest(write_request) => {

                }
                Event::NotifySubscribe(notify_subscribe) => {

                }
                Event::NotifyUnsubscribe => {

                }
            }

        }
    };


    let mut uuids: Vec<Uuid> = vec![];
    for s in &discovery.services {
        uuids.push(s.uuid.parse().unwrap())
    }

    println!("Starting advertising!");
    peripheral.start_advertising(&discovery.name, uuids.as_slice()).await?;


    println!("Finished!");
    Ok(())
}
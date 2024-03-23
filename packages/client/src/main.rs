use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::process::exit;

use bluster::gatt::characteristic::{Characteristic, Properties};
use bluster::gatt::characteristic;
use bluster::gatt::event::Event;
use bluster::gatt::service::Service;
use bluster::Peripheral;
use clap::Parser;
use futures::future::join_all;
use futures::{join, SinkExt, StreamExt};
use futures_channel::mpsc::{channel, Receiver, Sender};
use tonic;
use tonic::Status;
use tonic::transport::Channel;
use uuid::Uuid;

use ble_over_ip_proto::{DiscoverRequest, ReadRequest, SubscribeRequest, WriteRequest};
use ble_over_ip_proto::ble_proxy_client::BleProxyClient;
use ble_over_ip_proto::ble_proxy_server::BleProxy;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct ServerArgs {
    #[arg(short, long)]
    url: String,
}

#[derive(Debug, Clone)]
struct CharacteristicProxyError {
    status: Status,
}

impl Display for CharacteristicProxyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "received invalid status from proxy: {}", self.status)
    }
}

impl std::error::Error for CharacteristicProxyError {

}

struct CharacteristicHandler<'b> {
    characteristic: &'b ble_over_ip_proto::Characteristic,
    server: BleProxyClient<Channel>,
    sender: Sender<Event>,
    receiver: Receiver<Event>
}

impl<'b> CharacteristicHandler<'b> {
    async fn handle(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(event) = self.receiver.next().await {
            // process request from client device
            match event {
                Event::ReadRequest(_read_request) => {
                    let result = self.server.read(tonic::Request::new(ReadRequest {
                        characteristic_uuid: self.characteristic.uuid.clone(),
                    })).await;
                    return if result.is_ok() {
                        Ok(())
                    } else {
                        Err(Box::new(CharacteristicProxyError {
                            status: result.err().unwrap(),
                        }))
                    }
                }
                Event::WriteRequest(write_request) => {
                    let result = self.server.write(tonic::Request::new(WriteRequest {
                        characteristic_uuid: self.characteristic.uuid.clone(),
                        ack: !write_request.without_response,
                        data: write_request.data,
                    })).await;
                    return if result.is_ok() {
                        Ok(())
                    } else {
                        Err(Box::new(CharacteristicProxyError {
                            status: result.err().unwrap(),
                        }))                    }
                }
                Event::NotifySubscribe(mut notify_subscribe) => {
                    let result = self.server.subscribe(tonic::Request::new(SubscribeRequest {
                        characteristic_uuid: self.characteristic.uuid.clone(),
                        ack: false,
                    })).await;
                    return if result.is_ok() {
                        let response = result.unwrap();
                        let mut streaming = response.into_inner();
                        while let Some(Ok(event)) = streaming.next().await {
                            notify_subscribe.notification.send(event.data).await.unwrap()
                        }
                        Ok(())
                    } else {
                        Err(Box::new(CharacteristicProxyError {
                            status: result.err().unwrap(),
                        }))                           }
                }
                Event::NotifyUnsubscribe => {
                    let result = self.server.unsubscribe(tonic::Request::new(SubscribeRequest {
                        characteristic_uuid: self.characteristic.uuid.clone(),
                        ack: false,
                    })).await;
                    return if result.is_ok() {
                        Ok(())
                    } else {
                        Err(Box::new(CharacteristicProxyError {
                            status: result.err().unwrap(),
                        }))                           }
                }
            }

        }
        Ok(())
    }

    fn get_properties(& self) -> Properties {
        Properties::new(
            if self.characteristic.can_read {
                Some(
                    characteristic::Read(
                        characteristic::Secure::Insecure(self.sender.clone()))
                )
            } else { None },
            if self.characteristic.can_write {
                Some(
                    characteristic::Write::WithoutResponse(
                        self.sender.clone()
                    )
                )
            } else { None },
            if self.characteristic.can_subscribe {
                Some(
                    self.sender.clone()
                )
            } else { None },
            if self.characteristic.can_subscribe {
                Some(self.sender.clone())
            } else { None },
        )
    }
    fn new(characteristic: &'b ble_over_ip_proto::Characteristic, server: BleProxyClient<Channel>) -> CharacteristicHandler<'b> {
        let (sender, receiver) = channel(1);
        CharacteristicHandler {
            characteristic,
            server,
            sender,
            receiver,
        }
    }
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

    let mut handlers: Vec<CharacteristicHandler> = vec![];

    for s in &discovery.services {
        println!("├─ Service {}", s.uuid);
        let mut characteristics: HashSet<Characteristic> = HashSet::new();
        for c in &s.characteristics {
            let handler = CharacteristicHandler::new(c, client.clone());
            println!("├── Characteristic {}", c.uuid);
            characteristics.insert(Characteristic::new(
                c.uuid.parse().unwrap(),
                handler.get_properties(),
                None,
                HashSet::new(),
            ));
            handlers.push(handler);
        }
        let p_service = Service::new(
            s.uuid.parse().unwrap(),
            true,
            characteristics,
        );
        peripheral.add_service(&p_service).unwrap();
    }
    let mut uuids: Vec<Uuid> = vec![];
    for s in &discovery.services {
        uuids.push(s.uuid.parse().unwrap())
    }

    let adv_fut = async move {
        println!("Starting advertising!");
        peripheral.start_advertising(&discovery.name, uuids.as_slice()).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    let mut futures = vec![];
    for mut h in &mut handlers {
        futures.push(h.handle());
    }

    join!(adv_fut, join_all(futures));

    println!("Finished!");
    Ok(())
}
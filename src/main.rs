use async_std::prelude::*;
use async_std::task;

use async_std::io;
use async_std::io::BufReader;

use async_std::net::{TcpListener, TcpStream};

use async_std::sync::channel;
use async_std::sync::{Receiver, Sender};

use std::collections::HashMap;

struct Client {
    name: String,
    sender: Sender<String>,
}

struct Broker {
    clients: HashMap<String, Client>,
}

enum ClientEvent {
    Connect(Client),
    Message { name: String, msg: String },
    Disconnect { name: String },
}

async fn broker(mut incoming: Receiver<ClientEvent>) {
    let mut broker = Broker {
        clients: HashMap::new(),
    };

    while let Some(event) = incoming.next().await {
        match event {
            ClientEvent::Connect(c) => {
                broker.clients.insert(c.name.clone(), c);
            }
            ClientEvent::Message {
                name: sender_name,
                msg,
            } => {
                for (_, c) in broker
                    .clients
                    .iter()
                    .filter(|(name, _)| **name != sender_name)
                {
                    c.sender.send(format!("{}: {}\n", sender_name, msg)).await
                }
            }
            ClientEvent::Disconnect { name } => {
                broker.clients.remove(&name);
            }
        }
    }
}

async fn client(mut stream: TcpStream, broker_connection: Sender<ClientEvent>) -> io::Result<()> {
    println!("client connected");

    // read its name line
    let mut buffer = String::new();
    match stream.read_to_string(&mut buffer).await {
        Ok(_) => println!("their name is {}", buffer),
        Err(e) => {
            println!("couldn't read name. error {}. returning.", e);
            return Err(e);
        }
    }

    // register it with its broker

    // start task for incoming messages

    // start task for outgoing messages

    Ok(())
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();

        let (broker_sender, broker_receiver) = channel(10);

        task::spawn(broker(broker_receiver));

        while let Some(stream) = incoming.next().await {
            client(stream?, broker_sender.clone()).await?;
        }
        Ok(())
    })
}

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
    // HUH: what is this for? why does it contain a string and not a ClientEvent as with broker_connection?
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
                println!("[broker] got connection from client {}", c.name);
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


// HUH: var name mismatch (broker_connection vs broker_sender in fn call below) is confusing
// TODO: rename to client_handler
// TOOO: problem: if 2 clients connect and *then* staetheir names weird behaviour happens
async fn client(mut stream: TcpStream, broker_connection: Sender<ClientEvent>) -> io::Result<()> {
    println!("[client] user connected");
    let buf_read = BufReader::new(stream.clone());
    let mut lines = buf_read.lines();
    let mut user_name = String::new();

    // read its name line
    match lines.next().await {
        Some(Ok(input)) =>  {
            user_name = input;
            println!("[client] their name is {}", user_name);},
        _ => println!("[client] wtf")
    }

    // register client with its broker
    // TODO only do that if client creation was successful
    let (client_sender, mut client_receiver) = channel(1);
    let user = Client { name: user_name.clone(), sender: client_sender };
    let connect_event = ClientEvent::Connect(user);
    broker_connection.send(connect_event).await;
    println!("[client] registered {:?} with the broker", user_name);

    // pass messages from the user on to the broker
    // HUH: why the move in the given example?
    // -> wrote it without and got compiler error explaining why move was necessary.
    // Helpful learning experience!
    task::spawn(async move {
        while let Some(Ok(line)) = lines.next().await {
            let message_event = ClientEvent::Message{name: user_name.clone(), msg : line};
            broker_connection.send(message_event).await;
        }
    });

    // pass messages from the broker on to the user
    task::spawn(async move {
        while let Some(chat_msg) = client_receiver.next().await {
            println!("{}", chat_msg);
            // TODO handle reult more gracefully
            let _ = stream.write_all(chat_msg.as_bytes()).await;
        }
    });

    Ok(())
}

// HUH: supplying a simple chat client to test would be nice (since we know that nc is risky)
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

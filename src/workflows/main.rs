use std::error::Error;

use colored::Colorize;
use lapin::{Connection, ConnectionProperties};

const RABBITMQ_USERNAME: &str = "guest";
const RABBITMQ_PASSWORD: &str = "guest";
const RABBITMQ_ADDR: &str = "localhost:5672";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Don't understand why this is apparently optional
    let options = ConnectionProperties::default()
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let conn_str = format!("amqp://{RABBITMQ_USERNAME}:{RABBITMQ_PASSWORD}@{RABBITMQ_ADDR}");
    let conn_str_safe = format!("amqp://{RABBITMQ_USERNAME}:******@{RABBITMQ_ADDR}");
    println!("Connecting to {}", conn_str_safe.blue());

    let conn = Connection::connect(&conn_str, options).await?;
    if conn.status().errored() {
        println!("Could not connect to server: {:?}", conn.status().state());
    }
    let chan = conn.create_channel().await?;
    println!("Connection Status: {:?}", conn.status());

    Ok(())
}

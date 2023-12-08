
#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;
mod contract_queries;
use rocket::{State, Shutdown};
use rocket::fs::{relative, FileServer};
use rocket::form::Form;
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::tokio::select;
use std::sync::Arc;

use ethers::prelude::*;
use ethers::types::{H160, Address, U256};
use ethers::utils::to_checksum;

use std::fmt;
use std::str::FromStr;



abigen!(
    MyContract,
    "src/contracts/smart.abi.json",
    event_derives(serde::Deserialize, serde::Serialize)
);


#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate = "rocket::serde")]
struct Message { //
    #[field(validate = len(..30))]
    pub room: String,
    #[field(validate = len(..20))]
    pub username: String,
    pub message: String,
}

/// Returns an infinite stream of server-sent events. Each event is a message
/// pulled from a broadcast queue sent by the `post` handler.
#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

/// Receive a message from a form submission and broadcast it to any receivers.
#[post("/message", data = "<form>")]
async fn post(
    form: Form<Message>,
    queue: &State<Sender<Message>>,
    contract: &State<MyContract<SignerMiddleware<Provider<Http>, LocalWallet>>>,
) -> Result<(), String> {
    // Send the form message to the queue
    let _res = queue.send(form.into_inner());

    // Minting logic
    match contract.mint().send().await {
        Ok(tx) => {
            println!("Minted successfully: {:?}", tx);
            Ok(())
        },
        Err(e) => {
            eprintln!("Error while minting: {:?}", e);
            Err(format!("Error while minting: {}", e))
        }
    }
}


#[launch]
fn rocket() -> _ {
    // Setup Ethereum provider
    let provider = Provider::<Http>::try_from("https://goerli.infura.io/v3/708768d6c767447d86e4ddf69cb927ec")
        .expect("Provider not initialized");
    
    // Create a wallet from a private key (use environment variables or secure storage in production)
    let wallet = "b4a429f86181da6263455e8286785d886f60d8ce8c5a834b4e9500f76d2cd472"
                .parse::<LocalWallet>()
                .expect("Invalid private key")
                .with_chain_id(5u64);

    // Connect the wallet to the provider
    let client = SignerMiddleware::new(provider, wallet);

    // Wrap the client in an Arc for shared state management
    let arc_client = Arc::new(client);

    // Create contract instance
    let contract_address = "0x8247EC8a311669520ec0C272afBD763edDAf2521".parse::<Address>().expect("Invalid contract address");
    let contract = MyContract::new(contract_address, arc_client.clone());

    rocket::build()
        .manage(channel::<Message>(1024).0)
        .manage(contract)
        .mount("/", routes![post, events])
        .mount("/", FileServer::from(relative!("static")))
}

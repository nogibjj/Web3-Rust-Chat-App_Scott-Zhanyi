
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

// storerage
// s3
use s3::region::Region as S3Region;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use chrono::{Utc, DateTime};
use dotenv::dotenv;
use std::env;
// ipfs
use ipfs_api::IpfsClient;
use std::io::Cursor;
use ipfs_api::IpfsApi;



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
    pub wallet: String,
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


// Async function for IPFS upload
async fn upload_to_ipfs(data: Vec<u8>) -> Result<String, ipfs_api::Error> {
    let ipfs = IpfsClient::default();
    let data = Cursor::new(data);
    ipfs.add(data).await.map(|res| res.hash)
}


/// Receive a message from a form submission and broadcast it to any receivers.
#[post("/message", data = "<form>")]
async fn post(
    form: Form<Message>,
    queue: &State<Sender<Message>>,
    contract: &State<MyContract<SignerMiddleware<Provider<Http>, LocalWallet>>>,
) -> Result<(), String> {
    // Deserialize the form message
    let message = form.into_inner();
    let _res = queue.send(message.clone());

    // Serialize your message data
    let serialized_message = serde_json::to_string(&message).unwrap();
    
    // Generate a unique key using the current UTC timestamp
    let timestamp: DateTime<Utc> = Utc::now();
    let key = format!("messages/{}.json", timestamp.to_rfc3339());

    // Handle the Result from Credentials::new
    let credentials = Credentials::new(
        Some(env::var("AWS_ACCESS_KEY_ID").expect("AWS Access Key ID not set in env").as_ref()),
        Some(env::var("AWS_SECRET_ACCESS_KEY").expect("AWS Secret Access Key not set in env").as_ref()),
        None, None, None
    ).map_err(|e| e.to_string())?; // Convert any error to String

    let bucket = Bucket::new(
        "web3chatapp", // your bucket name
        S3Region::UsEast2, // replace with your bucket region
        credentials
    ).map_err(|e| e.to_string())?; // Convert any error to String

    // Upload to S3
    match bucket.put_object_with_content_type(&key, serialized_message.as_bytes(), "application/json").await {
        Ok(_) => println!("Uploaded message to S3"),
        Err(e) => eprintln!("Failed to upload message: {}", e),
    }


    // let ipfs = IpfsClient::new("127.0.0.1", 5001); // Replace with your IPFS server address and port

    // Serialize your message data
    let serialized_message = serde_json::to_string(&message).unwrap();
    let serialized_message_bytes = serialized_message.clone().into_bytes();

    // Spawn a blocking task to handle IPFS upload
    let ipfs_upload = tokio::task::spawn_blocking(move || {
        // Use tokio::runtime::Runtime to block on async function
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(upload_to_ipfs(serialized_message_bytes))
    });

    // Wait for the blocking task to complete
    match ipfs_upload.await {
        Ok(Ok(hash)) => println!("Uploaded to IPFS: {}", hash),
        Ok(Err(e)) => eprintln!("Failed to upload to IPFS: {}", e),
        Err(e) => eprintln!("IPFS upload task panicked: {:?}", e),
    }
    let wallet_address = H160::from_str(&message.wallet)
    .map_err(|e| format!("Invalid wallet address: {}", e))?;

    // Minting logic
    match contract.mint_user(wallet_address).send().await {
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
    dotenv().ok();
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

    // let aws_access_key_id = env::var("aws_access_key_id").expect("AWS Access Key ID not set in env");
    // let aws_secret_access_key = env::var("aws_secret_access_key").expect("AWS Secret Access Key not set in env");

    // Print the AWS credentials for debugging purposes
    match env::var("aws_access_key_id") {
        Ok(value) => println!("AWS Access Key ID: {}", value),
        Err(e) => println!("Couldn't read AWS Access Key ID: {}", e),
    }

    match env::var("aws_secret_access_key") {
        Ok(value) => println!("AWS Secret Access Key: {}", value),
        Err(e) => println!("Couldn't read AWS Secret Access Key: {}", e),
    }


    // Create contract instance
    let contract_address = "0x8247EC8a311669520ec0C272afBD763edDAf2521".parse::<Address>().expect("Invalid contract address");
    let contract = MyContract::new(contract_address, arc_client.clone());
    // let s3_client = S3Client::new(Region::UsEast2); 

    rocket::build()
        .manage(channel::<Message>(1024).0)
        .manage(contract)
        // .manage(s3_client)
        .mount("/", routes![post, events])
        .mount("/", FileServer::from(relative!("static")))
}

# Web3_rust_chatapp
This is a simple chat application built with Rust and Rocket framework on web3. 

# Installation
To run this application, you'll need to have Rust and Cargo installed. If you don't have them installed, you can download and install them from https://www.rust-lang.org/learn/get-started.

* The chat-app is for localhost:8000, you can try to use it locally. 
* The chat-app-rust is using the shuttle to deploy the chat-app.

Once you have Rust and Cargo installed, you can clone this repository and run the following command to start the server:
This will start the server on port 8000 by default. You can access the chat app by visiting http://localhost:8000 in your web browser.

# Development Process
The development process for this chat app involved the following steps:

* Setting up a new Rocket project using cargo new
* Adding dependencies to the Cargo.toml file for Rocket, Diesel, and other necessary packages.
* Setting up the database using Diesel migrations and SQLite.
* Creating the necessary models, routes, and controllers for the chat app.
* Implementing WebSocket support using the shuttle service.
* Testing the application and fixing any bugs or errors.
* deploy the application on web3 using wasmd. 




# Functionality
The chat app allows users to:

* Done:
1. Create a new user name.
2. Join or create chat rooms.
3. Send and receive messages in real time using WebSockets.
4. View a list of online users in each chat room.

* In process:
1. ChatGPT in the chatting room to answer questions. 
2. using IPFS to share file and documents. 

# Packages Used
The following packages were used to build this chat app:
> rocket = {version = "0.5.0-rc.1", features = ["json"]}

> time = "0.3.15"

> shuttle-service = { version = "0.11.0", features = ["web-rocket"] }

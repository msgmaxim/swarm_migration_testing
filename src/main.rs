#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate log;

extern crate env_logger;
extern crate log4rs;

mod client;
mod rpc_server;
mod swarms;
mod test_context;
mod tests;

use rand::prelude::*;
use std::io::prelude::*;
use swarms::*;

use rpc_server::Blockchain;
use std::sync::{Arc, Mutex};

fn print_sn_data(swarm: &Swarm) {
    for sn in &swarm.nodes {
        println!("[{}]", &sn.port);

        let messages = client::request_all_messages(&sn.port);

        for msg in messages {
            println!("  {} {}", msg.pk, msg.data);
        }
    }
}

fn send_req_to_quit(sn: &ServiceNode) -> Result<(), ()> {
    let target = "/quit";

    let addr = "https://localhost:".to_owned() + &sn.port + target;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build().unwrap();

    if let Err(_e) = client.post(&addr).send() {
        error!("could not send /quit request to a node at {}", &sn.port);
        Err(())
    } else {
        info!("quitting {}", &sn.port);
        Ok(())
    }
}

fn gracefully_exit(bc: &Arc<Mutex<Blockchain>>) {
    let sm = &mut bc.lock().unwrap().swarm_manager;

    sm.quit_children();

    // Not sure how to stop rpc the server,
    // or whether that is even necessary
    std::process::exit(0);
}

fn from_mins(mins : u64) -> std::time::Duration {
    let secs = mins * 60;
    std::time::Duration::from_secs(secs)
}

fn main() {
    // Overwrite logs with every run
    let path = std::path::Path::new("log");
    if path.exists() {
        std::fs::remove_dir_all("log").expect("could not remove 'log' directory");
    }

    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    let path = std::path::Path::new("playground");
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("Could not remove existing 'playground' directory");
    }

    let swarm_manager = SwarmManager::new();
    let blockchain = Blockchain::new(swarm_manager);
    let blockchain = Arc::new(Mutex::new(blockchain));

    // start RPC server
    let server_thread = rpc_server::start_http_server(&blockchain);

    let bc = Arc::clone(&blockchain);

    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        println!(""); // go to next line
        gracefully_exit(&bc);
    })
    .expect("error handling Ctrl+C handler");

        // Note: currently the test works fine to 15 minutes

    let options = tests::TestOptions {
        reliable_snodes: true,
        duration : std::time::Duration::from_secs(20),
        block_interval : std::time::Duration::from_secs(2),
        message_interval : std::time::Duration::from_millis(50),
    };

    let long_test_opt = tests::TestOptions {
        reliable_snodes: true,
        duration: from_mins(5),
        block_interval: std::time::Duration::from_secs(10),
        message_interval: std::time::Duration::from_millis(50),
    };

    // tests::async_test(&blockchain);

    // Note: when testing long-polling, need to
    // modify client request to use the right header
    // tests::long_polling(&blockchain);

    // tests::one_node_big_data(&blockchain);
    // tests::test_bootstrapping_peer_big_data(&blockchain);
    // tests::test_bootstrapping_swarm_big_data(&blockchain);
    // tests::single_node_one_message(&blockchain);
    // tests::single_swarm_one_message(&blockchain);
    // tests::sinlge_swarm_joined(&blockchain);
    // tests::multiple_swarms_static(&blockchain);
    // tests::test_dissolving(&blockchain);
    // tests::test_retry_batches(&blockchain);
    // tests::test_retry_singles(&blockchain);
    // tests::peer_testing(&blockchain);
    tests::test_blocks(&blockchain, &options);
    // tests::test_persistent_blocks(&blockchain, &options);


    let stdin = std::io::stdin();
    let mut iterator = stdin.lock().lines();

    let mut rng = StdRng::seed_from_u64(0);
    loop {
        let command = iterator.next().unwrap().unwrap();

        if command == "quit" || command == "q" {
            println!("terminating...");
            break;
        }

        if command == "test" {
            let sm = &blockchain.lock().unwrap().swarm_manager;

            for s in &sm.swarms {
                println!("          ___swarm {}___", s.swarm_id);
                print_sn_data(s);
            }
        }

        if command == "send" {
            // For now: send a random message to a random PK
            if client::send_random_message(&blockchain.lock().unwrap().swarm_manager, &mut rng)
                .is_err()
            {
                eprintln!("got error sending messages");
            }
        }
    }

    println!("waiting for service nodes to finish");

    gracefully_exit(&blockchain);

    // Will never be reached, just in case
    server_thread.join().unwrap();
}

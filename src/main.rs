#[macro_use]
extern crate serde_derive;

#[macro_use]
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

use std::sync::{Arc, Mutex};
use test_context::TestContext;
use rpc_server::Blockchain;

fn print_sn_data(sm: &SwarmManager, swarm: &Swarm) {
    for sn in &swarm.nodes {
        println!("[{}]", &sn.ip);

        let messages = client::request_all_messages(&sn.ip);

        for msg in messages {
            println!("  {} {}", msg.pk, msg.data);
        }
    }
}

fn send_req_to_purge(sn: &ServiceNode) {
    let target = "/purge";
    let addr = "http://localhost:".to_owned() + &sn.ip + target;

    let client = reqwest::Client::new();

    if let Err(_) = client.post(&addr).send() {
        error!("could not send /purge request to a node at {}", &sn.ip);
    } else {
        warn!("purging {}", &sn.ip);
    }
}

fn send_req_to_quit(sn: &ServiceNode) {
    let target = "/quit";
    let addr = "http://localhost:".to_owned() + &sn.ip + target;

    let client = reqwest::Client::new();

    if let Err(_) = client.post(&addr).send() {
        error!("could not send /quit request to a node at {}", &sn.ip);
    } else {
        warn!("quitting {}", &sn.ip);
    }
}

fn gracefully_exit(bc : & Arc<Mutex<Blockchain>>) {
    let sm = &mut bc.lock().unwrap().swarm_manager;
    print!("Quitting {} nodes...", sm.children.len());
    for sn in sm.swarms.iter() {
        for node in sn.nodes.iter() {
            send_req_to_quit(&node);
        }
    }

    for sn in sm.children.iter_mut() {
        sn.wait().unwrap();
    }

    println!("done");

    // Not sure how to stop rpc the server,
    // or whether that is even necessary
    std::process::exit(0);
}

fn main() {
    // Overwrite logs with every run
    let path = std::path::Path::new("log");
    if path.exists() {
        std::fs::remove_dir_all("log").expect("could not remove 'log' directory");
    }

    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    let path = std::path::Path::new("playground");
    std::fs::remove_dir_all(&path).expect("Could not remove existing 'playground' directory");

    let mut swarm_manager = SwarmManager::new();
    let blockchain = rpc_server::Blockchain::new(swarm_manager);
    let blockchain = Arc::new(Mutex::new(blockchain));

    let bc = Arc::clone(&blockchain);

    // start RPC server
    let server_thread = std::thread::spawn(move || {
        rpc_server::start_http_server(bc);
    });

    let bc = Arc::clone(&blockchain);

    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        println!(""); // go to next line
        gracefully_exit(&bc);
    })
    .expect("error handling Ctrl+C handler");

    // tests::async_test(&blockchain);
    // tests::long_polling(&blockchain);
    // tests::one_node_big_data(&blockchain);
    // tests::test_bootstrapping_peer_big_data(&blockchain);
    // tests::test_bootstrapping_swarm_big_data(&blockchain);
    // tests::single_node_one_message(&blockchain);
    // tests::single_swarm_one_message(&blockchain);
    // tests::sinlge_swarm_joined(&blockchain);
    // tests::test_dissolving(&blockchain);
    // tests::test_retry_batches(&blockchain);
    // tests::test_retry_singles(&blockchain);
    // tests::large_test(&blockchain);
    tests::test_blocks(&blockchain);

    let stdin = std::io::stdin();
    let mut iterator = stdin.lock().lines();

    let mut rng = StdRng::seed_from_u64(0);
    loop {
        let command = iterator.next().unwrap().unwrap();

        if command == "quit" || command == "q" {
            println!("terminating...");
            break;
        }

        if command == "purge" {
            println!("purging...");

            for sn in blockchain.lock().unwrap().swarm_manager.swarms.iter() {
                for node in sn.nodes.iter() {
                    send_req_to_purge(&node);
                }
            }
        }

        if command == "test" {
            let sm = &blockchain.lock().unwrap().swarm_manager;

            for s in &sm.swarms {
                println!("          ___swarm {}___", s.swarm_id);
                print_sn_data(&sm, s);
            }
        }

        if command == "send" {
            // For now: send a random message to a random PK
            if client::send_random_message(&blockchain.lock().unwrap().swarm_manager, &mut rng)
                .is_err()
            {
                eprintln!("got error sending messages");
            }

            // for _ in 0..1000 {
            //     client::send_random_message(&blockchain.lock().unwrap().swarm_manager);
            // }

            // client::barrage_messages("5902");
        }
    }

    println!("waiting for service nodes to finish");

    gracefully_exit(&blockchain);

    // Will never be reached, just in case
    server_thread.join().unwrap();
}

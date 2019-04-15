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
mod tests;
mod test_context;

use std::io::prelude::*;
use rand::prelude::*;
use swarms::*;

use std::sync::{Arc, Mutex};
use test_context::TestContext;

fn print_sn_data(sm : &SwarmManager, swarm: &Swarm) {
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

fn main() {

    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    let mut swarm_manager = SwarmManager::new();
    let blockchain = rpc_server::Blockchain::new(swarm_manager);
    let blockchain = Arc::new(Mutex::new(blockchain));

    let bc = Arc::clone(&blockchain);

    // start RPC server
    let server_thread = std::thread::spawn(move || {
        rpc_server::start_http_server(bc);
    });

    tests::async_test(Arc::clone(&blockchain));
    // tests::long_polling(Arc::clone(&blockchain));
    // tests::one_node_big_data(Arc::clone(&blockchain));
    // tests::test_bootstrapping_peer_big_data(Arc::clone(&blockchain));
    // tests::test_bootstrapping_swarm_big_data(Arc::clone(&blockchain));
    // tests::single_node_one_message(Arc::clone(&blockchain));
    // tests::single_swarm_one_message(Arc::clone(&blockchain));
    // tests::sinlge_swarm_joined(Arc::clone(&blockchain));
    // tests::test_dissolving(Arc::clone(&blockchain));
    // tests::test_snode_disconnecting(Arc::clone(&blockchain));
    // tests::large_test(Arc::clone(&blockchain));
    // tests::test_blocks(Arc::clone(&blockchain));
    // blockchain.lock().unwrap().reset();


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
            if client::send_random_message(&blockchain.lock().unwrap().swarm_manager, &mut rng).is_err() {
                eprintln!("got error sending messages");
            }

            // for _ in 0..1000 {
            //     client::send_random_message(&blockchain.lock().unwrap().swarm_manager);
            // }

            // client::barrage_messages("5902");
        }
    }

    println!("waiting for service nodes to finish");

    let sm = &mut blockchain.lock().unwrap().swarm_manager;

    for sn in sm.swarms.iter() {
        for node in sn.nodes.iter() {
            send_req_to_quit(&node);
        }
    }

    for sn in sm.children.iter_mut() {
        sn.wait().unwrap();
    }

    println!("RPC server thread is still running");

    server_thread.join().unwrap();
}

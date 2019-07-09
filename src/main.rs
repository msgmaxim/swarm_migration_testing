#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate log;

extern crate env_logger;
extern crate log4rs;

mod blockchain;
mod client;
mod rpc_server;
mod swarms;
mod test_context;
mod service_node;
mod daemon;
mod tests;

use rand::prelude::*;
use std::io::prelude::*;
use swarms::*;

use service_node::ServiceNode;

use blockchain::Blockchain;
use daemon::BlockchainView;
use std::sync::{Arc, Mutex};
use test_context::TestContext;

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
        .build()
        .unwrap();

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

fn from_mins(mins: u64) -> std::time::Duration {
    let secs = mins * 60;
    std::time::Duration::from_secs(secs)
}

fn main() {
    let matches = clap::App::new("Migration Testing")
        .version("0.1")
        .arg(
            clap::Arg::with_name("binary-path")
                .help("Path to the Storage Server binary to test")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let bin_path = matches
        .value_of("binary-path")
        .expect("no value for binary-path");
    println!("Using path: {}", bin_path);

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

    let blockchain = Blockchain::new(&bin_path);
    let blockchain = Arc::new(Mutex::new(blockchain));

    let ctx = TestContext::new(Arc::clone(&blockchain));
    let ctx = Arc::new(Mutex::new(ctx));

    // Create multiple views into the blockchain and create different
    // server instances for each of them

    let view_1 = BlockchainView::new(&blockchain);
    // start RPC server
    let server_thread = rpc_server::start_http_server(view_1);



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
        duration: std::time::Duration::from_secs(20),
        block_interval: std::time::Duration::from_secs(2),
        message_interval: std::time::Duration::from_millis(50),
    };

    let _long_test_opt = tests::TestOptions {
        reliable_snodes: true,
        duration: from_mins(5),
        block_interval: std::time::Duration::from_secs(10),
        message_interval: std::time::Duration::from_millis(50),
    };


    // tests::async_test(&ctx);

    // Note: when testing long-polling, need to
    // modify client request to use the right header
    // tests::long_polling(&ctx);

    // tests::one_node_big_data(&ctx);
    // tests::test_bootstrapping_peer_big_data(&ctx);
    // tests::test_bootstrapping_swarm_big_data(&ctx);
    // tests::single_node_one_message(&ctx);
    // tests::single_swarm_one_message(&ctx);
    // tests::sinlge_swarm_joined(&ctx);
    // tests::multiple_swarms_static(&ctx);
    // tests::test_dissolving(&ctx);
    // tests::test_retry_batches(&ctx);
    // tests::test_retry_singles(&ctx);
    // tests::test_blocks(&ctx, &options);
    tests::test_persistent_blocks(&ctx, &options);


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

            let swarms = ctx.lock().unwrap().get_swarms();

            for s in &swarms {
                println!("          ___swarm {}___", s.swarm_id);
                print_sn_data(s);
            }
        }

        if command == "send" {
            // For now: send a random message to a random PK
            ctx.lock().unwrap().send_random_message();
        }
    }

    println!("waiting for service nodes to finish");

    gracefully_exit(&blockchain);

    // Will never be reached, just in case
    server_thread.join().unwrap();
}

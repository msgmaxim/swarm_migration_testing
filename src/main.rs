#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

extern crate env_logger;

mod client;
mod rpc_server;
mod swarms;
mod test_context;

use std::io::prelude::*;
use swarms::*;

use std::sync::{Arc, Mutex};

#[allow(dead_code)]
fn prepare_testing() {
    let cur_dir = std::env::current_dir().unwrap();

    let playground_path = cur_dir.join("playground");

    std::fs::remove_dir_all(&playground_path)
        .map_err(|e| {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("error: {}", e);
            }
        })
        .ok();

    std::fs::create_dir(&playground_path).unwrap();
}

fn print_sn_data(swarm: &Swarm) {
    for sn in &swarm.nodes {
        println!("[{}]", &sn.ip);

        let mut path = String::new();

        path += &sn.ip;
        path += "/db.txt";

        let path = std::path::Path::new(&path);

        if let Ok(mut file) = std::fs::File::open(path) {
            let mut contents = String::new();

            file.read_to_string(&mut contents).unwrap();

            for line in contents.lines() {
                println!("  {}", line);
            }
        } else {
            println!("  <no file>");
        }
    }
}

fn test3(bc: Arc<Mutex<rpc_server::Blockchain>>) {
    let sm = &mut bc.lock().unwrap().swarm_manager;

    let ctx = test_context::TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    sm.add_swarm(&["5901", "5904"]);
    sm.add_swarm(&["5902", "5905"]);
    sm.add_swarm(&["5903", "5906"]);

    let bc = Arc::clone(&bc);
    let ctx = Arc::clone(&ctx);

    std::thread::spawn(move || {
        // give SNs some time to initialize their servers
        std::thread::sleep(std::time::Duration::from_millis(200));

        ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "This is a multi word message. It will move.");
        ctx.lock().unwrap().send_message("7266a21a81e6c820f803fa3e93e0d4485edced56dc220f4318dc9310a977a0ab", "No_move_(1)");

        // Add another swarm after some short period of time
        std::thread::sleep(std::time::Duration::from_millis(5000));
        bc.lock().unwrap().swarm_manager.add_swarm(&["5907", "5908"]);

        std::thread::sleep(std::time::Duration::from_millis(300));

        ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "No_move_(2)");

        // check that the messages are available

        std::thread::sleep(std::time::Duration::from_millis(3000));

        ctx.lock().unwrap().check_messages();

    });
}


fn test_dissolving(bc: Arc<Mutex<rpc_server::Blockchain>>) {

    let sm = &mut bc.lock().unwrap().swarm_manager;

    let ctx = test_context::TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    ctx.lock().unwrap().add_snode("5901");

    sm.add_swarm(&["5901"]);
    sm.add_swarm(&["5902"]);
    sm.add_swarm(&["5903"]);
    sm.add_swarm(&["5904"]);
    sm.add_swarm(&["5905"]);

    let bc = Arc::clone(&bc);

    std::thread::spawn(move || {
        // give SNs some time to initialize their servers
        std::thread::sleep(std::time::Duration::from_millis(200));

        ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A");
        ctx.lock().unwrap().send_message("1efe23d3a5445abe6f7a74c0601f352699d82a33020defa8c6f72f9cd1d403d3", "B");
        ctx.lock().unwrap().send_message("18b593e832ffda161c20a5daf842ab787ee7181a369ff7034fe80fb2774e0664", "C");
        ctx.lock().unwrap().send_message("51fcd662950d931d7a64e21378dd86c1505126612caf173d2ffb761e41f39aca", "D");

        // The messages will go to swarm 3, dissolve it
        &bc.lock().unwrap().swarm_manager.dissolve_swarm(3);

        std::thread::sleep(std::time::Duration::from_millis(300));

        // test that swarm 9223372036854775807 has messages A and D,
        // and that swarm 0 has messages B and C
        ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", "A2");
        ctx.lock().unwrap().send_message("1efe23d3a5445abe6f7a74c0601f352699d82a33020defa8c6f72f9cd1d403d3", "B2");
        ctx.lock().unwrap().send_message("18b593e832ffda161c20a5daf842ab787ee7181a369ff7034fe80fb2774e0664", "C2");
        ctx.lock().unwrap().send_message("51fcd662950d931d7a64e21378dd86c1505126612caf173d2ffb761e41f39aca", "D2");

        std::thread::sleep(std::time::Duration::from_millis(2000));

        ctx.lock().unwrap().check_messages();

    });

}


fn large_test(bc: Arc<Mutex<rpc_server::Blockchain>>) {

    // start with a limited pool of public keys
    let pks = [
        "ef57493cd18b632c7d4351f1bc91c23f62f933479e9fd5a976a6f2912edf71a3",
        "f63ca4260ffae0512e4f9f84435d7d76e41609d1ccf61507d67f9337c36c11a8",
        "3e96e57bdf99e68de8c88bc20ed4ba32e122194ea957dbc1f9417cdca28fd77",
        "ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", // this one should belong to swarm 3 after it is added
        "648763a60ef63e49988a3e3934338745ff45d6b8410181eab58caf92ec97ae27",
    ];


    let sm = &mut bc.lock().unwrap().swarm_manager;

    let ctx = test_context::TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    sm.add_swarm(&["5901"]);
    sm.add_swarm(&["5902"]);
    sm.add_swarm(&["5903"]);
    sm.add_swarm(&["5904"]);
    sm.add_swarm(&["5905"]);

    let bc = Arc::clone(&bc);

    // TODO

    std::thread::spawn(move || {
        // give SNs some time to initialize their servers
        std::thread::sleep(std::time::Duration::from_millis(200));


        // Construct an unreasonably large message:
        let mut large_msg = String::new();

        /// NOTE: Our server fails on this (Error(9): body limit exceeded)
        for _ in 0..200000 {
            large_msg.push_str("012345657");
        }

        ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", &large_msg);

        std::thread::sleep(std::time::Duration::from_millis(2000));

        ctx.lock().unwrap().check_messages();

    });


}

fn test_with_wierd_clients(bc: Arc<Mutex<rpc_server::Blockchain>>) {

    let sm = &mut bc.lock().unwrap().swarm_manager;

    let ctx = test_context::TestContext::new(Arc::clone(&bc));
    let ctx = Arc::new(Mutex::new(ctx));

    let bc = Arc::clone(&bc);

    sm.add_swarm(&["5901"]);

    std::thread::spawn(move || {
        // give SNs some time to initialize their servers
        std::thread::sleep(std::time::Duration::from_millis(200));


        // Construct an unreasonably large message:
        let mut large_msg = String::new();

        /// NOTE: Our server fails on this (Error(9): body limit exceeded)
        for _ in 0..200000 {
            large_msg.push_str("012345657");
        }

        ctx.lock().unwrap().send_message("ba0b9f5d5f82231c72696d12bb7cbaef3da3670a59c831b5b402986f9dcc3351", &large_msg);

        std::thread::sleep(std::time::Duration::from_millis(2000));

        ctx.lock().unwrap().check_messages();

    });

}

use tokio::prelude::*;

#[allow(dead_code)]
fn test_tokio() {
    let fut = tokio::timer::Delay::new(
        std::time::Instant::now() + std::time::Duration::from_millis(1000),
    );

    let fut = fut
        .and_then(|_| {
            println!("time out!");
            Ok(())
        })
        .map_err(|_e| panic!("timer failed"));

    tokio::run(fut);
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
    env_logger::init();

    let swarm_manager = SwarmManager::new();
    let blockchain = rpc_server::Blockchain::new(swarm_manager);
    let blockchain = Arc::new(Mutex::new(blockchain));

    let bc = Arc::clone(&blockchain);

    // start RPC server
    let server_thread = std::thread::spawn(move || {
        rpc_server::start_http_server(bc);
    });

    // let mut snodes = test(Arc::clone(&blockchain));
    test_with_wierd_clients(Arc::clone(&blockchain));

    let stdin = std::io::stdin();
    let mut iterator = stdin.lock().lines();

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
            client::send_random_message(&blockchain.lock().unwrap().swarm_manager);
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

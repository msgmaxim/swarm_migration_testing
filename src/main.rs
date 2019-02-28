#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

extern crate env_logger;

use rand::Rng;

mod rpc_server;
mod swarms;

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

#[allow(dead_code)]
fn spawn_service_node(
    path_to_bin: &std::path::Path,
    sn: &ServiceNode,
) -> Option<std::process::Child> {
    let mut server_process = std::process::Command::new(&path_to_bin);

    let path = std::path::Path::new(&sn.ip);

    if !path.exists() {
        std::fs::create_dir(&path).unwrap();
    }

    server_process.current_dir(&path);

    {
        let err_path = path.join("stderr.txt");

        let stderr_file = std::fs::File::create(&err_path).unwrap();

        let stdout_file = stderr_file.try_clone().unwrap();

        server_process.stderr(std::process::Stdio::from(stderr_file));
        server_process.stdout(std::process::Stdio::from(stdout_file));
    }

    server_process.arg("0.0.0.0");
    server_process.arg(sn.ip.to_string());

    match server_process.spawn() {
        Ok(child) => Some(child),
        Err(e) => {
            eprintln!("error spawning process: {}", e);
            None
        }
    }
}

fn spawn_init_swarms(sm: &SwarmManager, path_to_bin: &std::path::Path) -> Vec<std::process::Child> {
    let mut snodes = vec![];

    for swarm in sm.swarms.iter() {
        for sn in &swarm.nodes {
            if let Some(node) = spawn_service_node(&path_to_bin, &sn) {
                snodes.push(node);
            }
        }
    }

    snodes
}

fn test_sn_data(swarm: &Swarm) {
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

fn send_random_message(sm : &SwarmManager) {

    // generate random PK
    // For now, PK is a random 256 bit string

    let mut rng = rand::thread_rng();

    let n1 = rng.gen::<u64>();
    let n2 = rng.gen::<u64>();
    let n3 = rng.gen::<u64>();
    let n4 = rng.gen::<u64>();

    let pk = format!("{:x}{:x}{:x}{:x}", n1, n2, n3, n4);
    println!("pk: {}", pk);

}


fn main() {
    env_logger::init();

    let mut swarm_manager = SwarmManager { swarms: vec![] };

    // Add initial swarms
    swarm_manager.add_swarm(&["5901", "5902", "5903", "5904"]);
    swarm_manager.add_swarm(&["5905", "5906"]);

    swarm_manager.add_swarm(&["5908"]);
    swarm_manager.add_swarm(&["5909"]);
    swarm_manager.add_swarm(&["5910"]);

    return;

    let blockchain = rpc_server::RpcServer::new(swarm_manager);

    let blockchain = Arc::new(Mutex::new(blockchain));

    let bc = Arc::clone(&blockchain);

    // start RPC server
    let server_thread = std::thread::spawn(move || {
        rpc_server::start_http_server(bc);
    });

    let config_copy = &blockchain.lock().unwrap().swarm_manager.clone();

    let server_path =
        std::path::Path::new("/Users/maxim/Work/loki-storage-server/build/httpserver");
    let mut snodes = spawn_init_swarms(&config_copy, server_path);

    let bc = Arc::clone(&blockchain);
    let path_to_bin = server_path.clone();

    // TODO: should use Tokio instead
    std::thread::spawn(move || {
        // create another swarm after 5 sec
        std::thread::sleep(std::time::Duration::from_secs(5));

        warn!("CREATING A NEW SNODE");

        let sn = ServiceNode {
            ip: "5907".to_owned(),
        };

        spawn_service_node(&path_to_bin, &sn);

        bc.lock().unwrap().swarm_manager.swarms[0].nodes.push(sn);
    });

    let stdin = std::io::stdin();
    let mut iterator = stdin.lock().lines();

    loop {
        let command = iterator.next().unwrap().unwrap();

        if command == "quit" || command == "q" {
            println!("terminating...");
            break;
        }

        if command == "test" {
            let swarm = &blockchain.lock().unwrap().swarm_manager.swarms[0];
            test_sn_data(swarm);
        }

        if command == "send" {

            // For now: send a random message to a random PK
            send_random_message(&blockchain.lock().unwrap().swarm_manager);
        }
    }


    println!("waiting for service nodes to finish");
    for sn in snodes.iter_mut() {
        sn.wait().unwrap();
    }

    println!("RPC server thread is still running");

    server_thread.join().unwrap();
}

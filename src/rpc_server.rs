use std::sync::{Arc, Mutex};

use crate::swarms::*;
use rand::prelude::*;
use std::fmt::{self, Debug, Display};

use std::fs::File;
use std::io::prelude::*;

pub struct KeyPair {
    pub pubkey: String,
    pub seckey: String,
}

pub struct Blockchain {
    pub swarm_manager: SwarmManager,
    height: u64,
    keypair_pool: Vec<KeyPair>,
    block_hash: String,
}

impl Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for swarm in &self.swarm_manager.swarms {
            if let Err(e) = write!(f, "[{}] ", swarm.nodes.len()) {
                return Err(e);
            }
        }

        Ok(())
    }
}

impl Debug for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for swarm in &self.swarm_manager.swarms {
            if let Err(e) = write!(f, "[{:?}] ", swarm) {
                return Err(e);
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Debug)]
struct ServiceNodeState {
    service_node_pubkey: String,
    secret_key: String,
    public_ip: String,
    storage_port: u16,
    swarm_id: u64,
}

#[derive(Serialize, Debug)]
struct SwarmResult {
    service_node_states: Vec<ServiceNodeState>,
    height: u64,
    block_hash: String,
}

#[derive(Serialize, Debug)]
struct SwarmResponse {
    result: SwarmResult,
}

fn gen_random_hash() -> String {
    let n1 = rand::thread_rng().gen::<u64>();
    let n2 = rand::thread_rng().gen::<u64>();
    let n3 = rand::thread_rng().gen::<u64>();
    let n4 = rand::thread_rng().gen::<u64>();

    format!("{:016x}{:016x}{:016x}{:016x}", n1, n2, n3, n4)
}

pub static RPC_PORT: u16 = 22029;

impl Blockchain {
    pub fn new(swarm_manager: SwarmManager) -> Blockchain {
        // 0 is used to indicate that SN haven't synced yet
        let height = 1;
        let block_hash = gen_random_hash();

        // read keys file
        let mut contents = String::new();
        let mut key_file = std::fs::File::open("keys.txt").expect("could not open key file");
        key_file.read_to_string(&mut contents).unwrap();
        println!("total keys: {}", contents.lines().count());
        let keypair_pool: Vec<KeyPair> = contents
            .lines()
            .map(|pair| {
                let mut pair = pair.split_whitespace();
                KeyPair {
                    seckey: pair.next().unwrap().to_owned(),
                    pubkey: pair.next().unwrap().to_owned(),
                }
            })
            .collect();

        Blockchain {
            swarm_manager,
            keypair_pool,
            height,
            block_hash,
        }
    }

    pub fn pop_keypair(&mut self) -> KeyPair {
        self.keypair_pool.pop().expect("Could not pop a key pair")
    }

    pub fn reset(&mut self) {
        self.swarm_manager.reset();
    }

    fn construct_swarm_json(&self) -> String {
        let mut sn_list = vec![];

        for swarm in &self.swarm_manager.swarms {
            for sn in &swarm.nodes {
                let service_node_pubkey = sn.pubkey.clone();
                let secret_key = sn.seckey.clone();
                let public_ip = String::from("localhost");
                let secret_key = sn.seckey.clone();
                let storage_port = sn.port.clone().parse::<u16>().unwrap();
                let swarm_id = swarm.swarm_id;
                sn_list.push(ServiceNodeState {
                    service_node_pubkey,
                    secret_key,
                    public_ip,
                    storage_port,
                    swarm_id,
                })
            }
        }

        let service_node_states = sn_list;

        let response = SwarmResponse {
            result: SwarmResult {
                service_node_states,
                height: self.height,
                block_hash: self.block_hash.clone(),
            },
        };

        serde_json::to_string(&response).expect("could not construct json")
    }

    fn construct_bc_test_json(&self) -> String {

        println!("construct bc test json");

        let res = serde_json::json!({
            "result": {
                "res_height": 123
            }
        });

        res.to_string()
    }

    fn process_json_rpc(&mut self, val: serde_json::Value) -> String {
        let mut res = String::new();

        if let Some(Some(method)) = val.get("method").map(|v| v.as_str()) {
            trace!("got json rcp request, method: {:?}", &method);

            match method {
                "get_service_nodes" => {
                    res = self.construct_swarm_json();
                }
                "perform_blockchain_test" => {
                    res = self.construct_bc_test_json();
                }
                _ => {
                    warn!("unknown method: <{}>", &method);
                }
            }
        }

        res
    }

    pub fn inc_block_height(&mut self) {
        self.height += 1;
        self.block_hash = gen_random_hash();
    }
}

/// Starts a new thread
pub fn start_http_server(blockchain: &Arc<Mutex<Blockchain>>) -> std::thread::JoinHandle<()> {
    let bc = Arc::clone(&blockchain);

    let thread = std::thread::spawn(move || {
        let server = simple_server::Server::new(move |req, mut res| {
            let req_body = String::from_utf8_lossy(req.body());

            let mut res_body = String::new();

            if req.uri() == "/json_rpc" {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&req_body) {
                    res_body = bc.lock().unwrap().process_json_rpc(val);
                } else {
                    warn!("invalid json: \n{:?}", &req_body);
                }
            }

            let final_res = res.body(res_body.as_bytes().to_vec()).unwrap();

            Ok(final_res)
        });

        let port = RPC_PORT.to_string();

        println!("starting RPC server on port {}...", &port);

        server.listen("0.0.0.0", &port);
    });

    thread
}

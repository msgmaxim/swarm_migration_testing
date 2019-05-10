use std::sync::{Arc, Mutex};

use crate::swarms::*;
use std::fmt::{self, Debug, Display};
use rand::prelude::*;

pub struct Blockchain {
    pub swarm_manager: SwarmManager,
    height : u64,
    block_hash : String,
}

impl Display for Blockchain {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{

        for swarm in &self.swarm_manager.swarms {
            if let Err(e) = write!(f, "[{}] ", swarm.nodes.len()) {
                return Err(e);
            }
        }

        Ok(())
    }

}

impl Debug for Blockchain {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{

        for swarm in &self.swarm_manager.swarms {
            if let Err(e) = write!(f, "[{:?}] ", swarm) {
                return Err(e);
            }
        }

        Ok(())
    }

}

#[derive(Serialize)]
struct ServiceNodeState {
    service_node_pubkey : String,
    swarm_id : u64,
}

#[derive(Serialize)]
struct SwarmResult {
    service_node_states : Vec<ServiceNodeState>,
    height : u64,
    block_hash : String
}

#[derive(Serialize)]
struct SwarmResponse {
    result : SwarmResult
}

fn gen_random_hash() -> String {

    let n1 = rand::thread_rng().gen::<u64>();
    let n2 = rand::thread_rng().gen::<u64>();
    let n3 = rand::thread_rng().gen::<u64>();
    let n4 = rand::thread_rng().gen::<u64>();

    format!("{:016x}{:016x}{:016x}{:016x}", n1, n2, n3, n4)

}

impl Blockchain {
    pub fn new(swarm_manager: SwarmManager) -> Blockchain {

        // 0 is used to indicate that SN haven't synced yet
        let height = 1;
        let block_hash =  gen_random_hash();
        Blockchain { swarm_manager, height, block_hash }
    }

    pub fn reset(&mut self) {
        self.swarm_manager.reset();
    }

    fn construct_swarm_json(&self) -> String {

        let mut sn_list = vec![];

        for swarm in &self.swarm_manager.swarms {
            for sn in &swarm.nodes {
                let service_node_pubkey = sn.ip.clone();
                let swarm_id = swarm.swarm_id;
                sn_list.push( ServiceNodeState { service_node_pubkey, swarm_id } )
            }
        }

        let service_node_states = sn_list;

        let response = SwarmResponse { result : SwarmResult { service_node_states, height: self.height, block_hash: self.block_hash.clone() } };

        serde_json::to_string(&response).expect("could not construct json")
    }

    fn process_json_rpc(&mut self, val: serde_json::Value) -> String {
        let mut res = String::new();

        if let Some(Some(method)) = val.get("method").map(|v| v.as_str()) {
            trace!("got json rcp request, method: {:?}", &method);

            match method {
                "get_service_nodes" => {
                    res = self.construct_swarm_json();
                },
                "get_swarm_by_pk" => {
                    warn!("TODO: handle get_swarm_by_pk");
                },
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
pub fn start_http_server(blockchain: &Arc<Mutex<Blockchain>>) -> std::thread::JoinHandle<()>
{

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

        println!("starting RPC server on port 22023...");

        server.listen("0.0.0.0", "22023");

    });

    thread
}

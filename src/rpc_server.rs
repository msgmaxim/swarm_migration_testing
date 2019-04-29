pub struct Blockchain {
    pub swarm_manager: SwarmManager,
    pub height : u64
}

use std::sync::{Arc, Mutex};

use crate::swarms::*;
use std::fmt::{self, Debug, Display};

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
struct ServiceNodeInfo {
    swarm_id : u64
}

#[derive(Serialize)]
struct ServiceNodeSwarmData {
    pubkey : String,
    info : ServiceNodeInfo,
}

#[derive(Serialize)]
struct SwarmResult {
    service_node_states : Vec<ServiceNodeSwarmData>,
    height : u64
}

#[derive(Serialize)]
struct SwarmResponse {
    result : SwarmResult
}

impl Blockchain {
    pub fn new(swarm_manager: SwarmManager) -> Blockchain {

        // 0 is used to indicate that SN haven't synced yet
        let height = 1;
        Blockchain { swarm_manager, height }
    }

    pub fn reset(&mut self) {
        self.swarm_manager.reset();
    }

    fn construct_swarm_json(&self) -> String {

        let mut sn_list = vec![];

        for swarm in &self.swarm_manager.swarms {
            for sn in &swarm.nodes {
                sn_list.push( ServiceNodeSwarmData{ pubkey : sn.ip.clone(), info : ServiceNodeInfo { swarm_id : swarm.swarm_id } } )
            }
        }

        let service_node_states = sn_list;

        let response = SwarmResponse { result : SwarmResult { service_node_states, height: self.height } };

        serde_json::to_string(&response).expect("could not construct json")
    }

    fn process_json_rpc(&mut self, val: serde_json::Value) -> String {
        let mut res = String::new();

        if let Some(Some(method)) = val.get("method").map(|v| v.as_str()) {
            info!("method is: {:?}", &method);

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

        warn!("{}", &res);

        res
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

        println!("starting RPC server on port 38157...");

        server.listen("0.0.0.0", "38157");

    });

    thread
}

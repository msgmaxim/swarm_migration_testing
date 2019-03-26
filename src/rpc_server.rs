pub struct Blockchain {
    pub swarm_manager: SwarmManager,
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

impl Blockchain {
    pub fn new(swarm_manager: SwarmManager) -> Blockchain {
        Blockchain { swarm_manager }
    }

    pub fn reset(&mut self) {
        self.swarm_manager.reset();
    }

    fn process_json_rpc(&mut self, val: serde_json::Value) -> String {
        let mut res = String::new();

        if let Some(Some(method)) = val.get("method").map(|v| v.as_str()) {
            info!("method is: {:?}", &method);

            match method {
                "get_service_nodes" => {
                    for swarm in &self.swarm_manager.swarms {
                        res.push_str(&swarm.swarm_id.to_string());
                        res.push_str(" ");
                        for sn in &swarm.nodes {
                            res.push_str(&sn.ip);
                            res.push_str(" ");
                        }

                        res.push_str("\n");
                    }
                },
                "get_swarm_by_pk" => {
                    warn!("TODO: handle get_swarm_by_pk");
                },
                _ => {
                    warn!("unknown method: <{}>", &method);
                }
            }
        }

        info!("{:?}", &res);

        res
    }
}

pub fn start_http_server(bc: Arc<Mutex<Blockchain>>) {
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

    println!("starting RPC server on port 7777...");

    server.listen("0.0.0.0", "7777");
}

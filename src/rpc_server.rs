use std::sync::{Arc, Mutex};

use crate::blockchain::Blockchain;

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
    target_height: u64,
    block_hash: String,
    hardfork: u8,
}

#[derive(Serialize, Debug)]
struct RpcResponse<Type> {
    result: Type,
}

pub static RPC_PORT: u16 = 22029;

fn construct_swarm_json(bc: &Blockchain) -> String {
    let mut sn_list = vec![];
    for swarm in &bc.swarm_manager.swarms {
        for sn in &swarm.nodes {
            let service_node_pubkey = sn.pubkey.clone();
            let secret_key = sn.seckey.clone();
            let public_ip = String::from("localhost");
            let storage_port = sn.port.parse::<u16>().unwrap();
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

    let response = RpcResponse::<SwarmResult> {
        result: SwarmResult {
            service_node_states,
            height: bc.get_height(),
            target_height: bc.get_target_height(),
            block_hash: bc.get_block_hash().clone(),
            hardfork: 12,
        },
    };

    serde_json::to_string(&response).expect("could not construct json")
}

fn construct_bc_test_json() -> String {

    let res = serde_json::json!({
        "result": {
            "res_height": 123
        }
    });

    res.to_string()
}

fn construct_ping_json() -> String {

    let res = serde_json::json!({
        "result": {
            "status": "OK"
        }
    });

    res.to_string()

}

fn process_json_rpc(bc: &Blockchain, req_body: serde_json::Value) -> String {
    let mut res = String::new();

    if let Some(Some(method)) = req_body.get("method").map(|v| v.as_str()) {
        trace!("got json rcp request, method: {:?}", &method);

        match method {
            "get_n_service_nodes" => {
                res = construct_swarm_json(&bc);
            }
            "perform_blockchain_test" => {
                res = construct_bc_test_json();
            }
            "storage_server_ping" => {
                res = construct_ping_json();
            }
            _ => {
                warn!("unknown method: <{}>", &method);
            }
        }
    }

    res

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
                    res_body = process_json_rpc(&bc.lock().unwrap(), val);
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

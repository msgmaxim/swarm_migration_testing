use crate::daemon::{BlockchainView, BlockchainViewable};

#[derive(Serialize, Debug)]
struct ServiceNodeState {
    service_node_pubkey: String,
    pubkey_x25519: String, // hex
    pubkey_ed25519: String, // hex
    operator_address: String,
    secret_key: String,
    public_ip: String,
    storage_port: u16,
    storage_lmq_port: u16,
    swarm_id: u64,
    funded: bool
}

#[derive(Serialize, Debug)]
struct SwarmResult {
    service_node_states: Vec<ServiceNodeState>,
    height: u64,
    target_height: u64,
    block_hash: String,
    hardfork: u8
}

#[derive(Serialize, Debug)]
struct RpcResponse<Type> {
    result: Type,
}

#[derive(Deserialize, Debug)]
struct GetNodesRequest {
    params: bool,
}

fn construct_swarm_json(bc_view: &BlockchainView) -> String {
    let mut sn_list = vec![];

    // bc_view needs get_swarms()
    for swarm in &bc_view.get_swarms() {
        for sn in &swarm.nodes {
            let service_node_pubkey = sn.pubkey.clone();
            let secret_key = sn.seckey.clone();
            let public_ip = String::from("localhost");
            let operator_address = String::from("test");
            let storage_port = sn.port.parse::<u16>().unwrap();
            // TODO: actually add this port to sn (and check that it is available)
            let storage_lmq_port = sn.port.parse::<u16>().unwrap() + 200;
            let swarm_id = swarm.swarm_id;
            let pubkey_x25519 = sn.pubkey_x25519.clone();
            let pubkey_ed25519 = sn.ed_keys.pubkey.clone();
            sn_list.push(ServiceNodeState {
                service_node_pubkey,
                pubkey_x25519,
                pubkey_ed25519,
                secret_key,
                public_ip,
                storage_port,
                storage_lmq_port,
                operator_address,
                swarm_id,
                funded: true
            })
        }
    }
    let service_node_states = sn_list;

    let response = RpcResponse::<SwarmResult> {
        result: SwarmResult {
            service_node_states,
            height: bc_view.get_height(),
            target_height: bc_view.get_target_height(),
            block_hash: bc_view.get_block_hash().clone(),
            hardfork: bc_view.get_hf()
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

fn construct_report_json() -> String {
    let res = serde_json::json!({
        "result": {
            "status": "OK"
        }
    });

    res.to_string()
}

fn process_json_rpc(bc_view: &BlockchainView, req_body: serde_json::Value) -> String {
    let mut res = String::new();

    if let Some(Some(method)) = req_body.get("method").map(|v| v.as_str()) {
        trace!("got json rcp request, method: {:?}", &method);

        match method {
            "get_n_service_nodes" => {

                let mut real_messenger = false;

                if let Some(params) = req_body.get("params") {

                    if let Some(Some(active_only)) = params.get("active_only").map(|v| v.as_bool()) {
                        if active_only {
                        
                            real_messenger = true;
                            println!("GET_N_SERVICE_NODES params, {:?}", params);
                        }
                    }
                }

                res = construct_swarm_json(&bc_view);

                if real_messenger {
                    dbg!(&res);
                }

            }
            "perform_blockchain_test" => {
                res = construct_bc_test_json();
            }
            "storage_server_ping" => {
                res = construct_ping_json();
            }
            "report_peer_storage_server_status" => {
                res = construct_report_json();
            }
            _ => {
                warn!("unknown method: <{}>", &method);
            }
        }
    }

    res

}

use std::io::Read;

pub fn start_http_server2(bc_view: BlockchainView, port: u16) -> std::thread::JoinHandle<()> {

    let thread = std::thread::spawn(move || {

        let addr = format!("0.0.0.0:{}", port);

        info!("Running RPC Server on {}", addr);
        
        rouille::start_server(addr, move |request| {

            let mut data = request.data().expect("data already retrieved?");

            let mut buf = Vec::new();
            match data.read_to_end(&mut buf) {
                Ok(_) => (),
                Err(_) => return rouille::Response::text("Failed to read body")
            };

            let req_body = String::from_utf8_lossy(&buf);

            let mut res_body = String::new();

            if request.url() == "/json_rpc" {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&req_body) {
                    res_body = process_json_rpc(&bc_view, val);
                } else {
                    warn!("invalid json: \n{:?}", &req_body);
                    println!("invalid json: \n{:?}", &req_body);
                }
            } else if request.url() == "/lsrpc" {
                println!("got an lsrpc request");
                res_body = "OK".to_owned();
            }

            rouille::Response::from_data("application/json", res_body.as_bytes())
        });


    });

    thread

}

#[allow(dead_code)]
/// Starts a new thread
pub fn start_http_server(bc_view: BlockchainView, port : u16) -> std::thread::JoinHandle<()> {

    let thread = std::thread::spawn(move || {
        let server = simple_server::Server::new(move |req, mut res| {
            let req_body = String::from_utf8_lossy(req.body());

            let mut res_body = String::new();

            if req.uri() == "/json_rpc" {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&req_body) {
                    res_body = process_json_rpc(&bc_view, val);
                } else {
                    warn!("invalid json: \n{:?}", &req_body);
                    println!("invalid json: \n{:?}", &req_body);
                }
            }

            res.header("Content-Type", "application/json");

            let final_res = res.body(res_body.as_bytes().to_vec()).unwrap();

            Ok(final_res)
        });

        let port = port.to_string();

        println!("starting RPC server on port {}...", &port);

        server.listen("0.0.0.0", &port);
    });

    thread
}

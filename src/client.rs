use crate::swarms::{PubKey, SwarmManager};
use rand::Rng;

// body: { // Encrypted with EphemKey
//   method: store,
//   args: {
//     pubKey: ...,
//     ttl: ...,
//     pow: ...,
//     timestamp: ...,
//     data: WebSocketRequestMessage // Encrypted with signal
// }

use std::io::prelude::*;

#[allow(non_snake_case)]
#[derive(Serialize)]
struct StoreArgs {
    pubKey : String,
    data: String
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct RetrieveArgs {
    pubKey : String,
}

#[derive(Serialize)]
struct StoreBody {

    method : String,
    args : StoreArgs,
}

#[derive(Serialize)]
struct RetrieveBody {

    method : String,
    args : RetrieveArgs,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageResponse {
    pub data : String
}

#[derive(Serialize, Deserialize, Debug)]
struct RetrieveResponse {

    messages : Vec<MessageResponse>
}

pub struct MessagePair {
    pub pub_key: String,
    pub message: String,
}

pub fn send_message(port: &str, pk: &str, msg: &str) {
    let target = "/v1/storage_rpc";
    let addr = "http://localhost:".to_owned() + port + target;

    let client = reqwest::Client::new();
    
    let msg = StoreBody {
        method : "store".to_owned(),
        args : StoreArgs {
            pubKey : pk.to_owned(),
            data : msg.to_owned()
        }
    };

    let msg = serde_json::to_string(&msg).unwrap();

    let req =  client
        .post(&addr)
        .header("X-Loki-recipient", pk.to_owned())
        .header("X-Loki-ttl", "86400")
        .header("X-Loki-ephemkey", "86400")
        .header("X-Loki-timestamp", "1540860811000")
        .body(msg);

    match req.send() {

        Ok(mut _res) => {

            // let mut body = String::new();
            // res.read_to_string(&mut body);

            // dbg!(&res);
            // dbg!(&body);

        },
        Err(e) => {
            error!("Error requesting messages: {}", e);
        }

    }

    // dbg!(&res);
}

pub fn request_messages(sm: &SwarmManager, pk: &str) -> Vec<MessageResponse> {

    let swarm_idx = sm.get_swarm_by_pk(&PubKey::new(pk).unwrap());

    // TODO: this should be a random snode instead (or all of them)!
    let sn = &sm.swarms[swarm_idx as usize].nodes[0];

    warn!("swarm id: {}", &sm.swarms[swarm_idx as usize].swarm_id);

    let target = "/v1/storage_rpc";
    let addr = "http://localhost:".to_owned() + &sn.ip + target;

    warn!("requesting messages from: {} [{}]", &sm.swarms[swarm_idx as usize].swarm_id, &sn.ip);

    let client = reqwest::Client::new();

    let msg = RetrieveBody {
        method : "retrieve".to_owned(),
        args : RetrieveArgs {
            pubKey : pk.to_owned()
        }
    };

    let msg = serde_json::to_string(&msg).unwrap();

    let req =  client
        .post(&addr)
        .header("X-Loki-recipient", pk.to_owned())
        .header("X-Loki-ttl", "86400")
        .header("X-Loki-ephemkey", "86400")
        .header("X-Loki-timestamp", "1540860811000")
        .body(msg);

    match req.send() {

        Ok(mut res) => {

            let mut body = String::new();
            res.read_to_string(&mut body).unwrap();

            if let Ok(body) = serde_json::from_str::<RetrieveResponse>(&body) {

                return body.messages;

            } else {

                error!("Could not parse message: {:?}", &body);
                return vec![];
            }

        },
        Err(e) => {
            error!("Error requesting messages: {}", e);
            return vec![];
        }

    }

}

pub fn send_message_to_pk(sm: &SwarmManager, pk_str: &str, msg: &str) {

    let pk = PubKey::new(&pk_str).unwrap();

    let swarm_idx = sm.get_swarm_by_pk(&pk);

    let sn = &sm.swarms[swarm_idx as usize].nodes[0];

    dbg!(&sm.swarms[swarm_idx as usize].swarm_id);

    send_message(&sn.ip, &pk_str, &msg);

}

pub fn send_random_message(sm: &SwarmManager) {
    // generate random PK
    // For now, PK is a random 256 bit string

    let pk = PubKey::gen_random();

    let pk_str = pk.to_string();

    println!("pk: {}", &pk_str);

    let swarm_idx = sm.get_swarm_by_pk(&pk);

    // TODO: use random sn (or multiple sns)

    let sn = &sm.swarms[swarm_idx as usize].nodes[0];

    let mut rng = rand::thread_rng();

    let num = rng.gen::<u16>();

    let msg = num.to_string();

    send_message(&sn.ip, &pk_str, &msg);

    println!("sending random message <{}> to {} to sn {} from swarm {}", msg, pk_str, &sn.ip, swarm_idx);
}
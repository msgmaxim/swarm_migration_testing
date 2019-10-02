use crate::swarms::{PubKey, SwarmManager};
use crate::service_node::ServiceNode;

use rand::prelude::*;
use std::io::prelude::*;

use hyper::rt::{self, Future};
use hyper::{Body};
use std::time::SystemTime;

#[allow(non_snake_case)]
#[derive(Serialize)]
struct StoreArgs {
    pubKey: String,
    ttl: String,
    nonce: String,
    timestamp: String,
    data: String,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct RetrieveArgs {
    pubKey: String,
    lastHash: String,
}

#[derive(Serialize)]
struct StoreBody {
    method: String,
    params: StoreArgs,
}

#[derive(Serialize)]
struct RetrieveBody {
    method: String,
    params: RetrieveArgs,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageResponse {
    pub data: String,
    pub hash: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageResponseFull {
    pub pk: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RetrieveResponseFull {
    messages: Vec<MessageResponseFull>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RetrieveResponse {
    messages: Vec<MessageResponse>,
}

pub fn make_random_message(rng : &mut StdRng) -> String {

    let num = rng.gen::<u64>();
    num.to_string() + &num.to_string() + &num.to_string()
}

use hyper::client::HttpConnector;

pub fn send_message_async(client: &hyper::Client<HttpConnector, Body>, port: &str, pk: &str, msg: &str) -> hyper::client::ResponseFuture {

    let pk = "05".to_owned() + &pk;
    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();

    let msg = StoreBody {
        method: "store".to_owned(),
        params: StoreArgs {
            pubKey: pk.clone(),
            ttl: "86400".to_owned(),
            nonce: "324324".to_owned(),
            timestamp: timestamp.to_string(),
            data: msg.to_owned(),
        },
    };

    let body = serde_json::to_string(&msg).unwrap();

    let target = "/storage_rpc/v1";
    let uri = "https://localhost:".to_owned() + port + target;
    let uri: hyper::Uri = uri.parse().unwrap();

    let mut req = hyper::Request::new(Body::from(body));
    *req.method_mut() = hyper::Method::POST;
    *req.uri_mut() = uri.clone();
    req.headers_mut().insert(
        "X-Loki-ephemkey", hyper::header::HeaderValue::from_str("86400").unwrap()
    );

    client.request(req)
}

pub fn send_message(port: &str, pk: &str, msg: &str) -> Result<(), ()> {
    let target = "/storage_rpc/v1";
    let addr = "https://localhost:".to_owned() + port + target;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build().expect("building certificate");

    // Prepend the two characters like signal does
    let pk = "05".to_owned() + &pk;

    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();

    let ttl = "86400000".to_owned(); // 1 day

    let msg = StoreBody {
        method: "store".to_owned(),
        params: StoreArgs {
            pubKey: pk.clone(),
            ttl,
            nonce: "324324".to_owned(),
            timestamp: timestamp.to_string(),
            data: msg.to_owned(),
        },
    };

    let msg = serde_json::to_string(&msg).expect("building json");

    let req = client
        .post(&addr)
        .header("X-Loki-ephemkey", "86400")
        .body(msg);

    match req.send() {
        Ok(res) => {

            if res.status().is_success() {
                Ok(())
            } else {

                error!("HTTP error: {:?}", res);
                Err(())
            }

        }
        Err(e) => {
            error!("Error storing messages: {}", e);
            Err(())
        }
    }

    // dbg!(&res);
}

pub fn request_all_messages(sn: &str) -> Vec<MessageResponseFull> {

    let target = "/retrieve_all/v1";
    let addr = "https://localhost:".to_owned() + &sn + target;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build().unwrap();
    let req = client.post(&addr);

    match req.send() {
        Ok(mut res) => {
            let mut body = String::new();
            res.read_to_string(&mut body).unwrap();

            if body == "" {
                return vec![];
            }

            if let Ok(body) = serde_json::from_str::<RetrieveResponseFull>(&body) {
                return body.messages;
            } else {
                error!("Could not parse message: {:?}", &body);
                return vec![];
            }
        }
        Err(e) => {
            error!("Error requesting messages: {}", e);
            return vec![];
        }
    }
}

pub fn request_messages(sn: &ServiceNode, pk: &str) -> Vec<MessageResponse> {
    request_messages_given_hash(&sn, &pk, "")
}

pub fn request_messages_given_hash(sn: &ServiceNode, pk: &str, last_hash: &str) -> Vec<MessageResponse> {

    let target = "/storage_rpc/v1";

    let addr = "https://localhost:".to_owned() + &sn.port + target;


    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build().unwrap();

    // Prepend the two characters like signal does
    let pk = "05".to_owned() + &pk;

    let msg = RetrieveBody {
        method: "retrieve".to_owned(),
        params: RetrieveArgs {
            pubKey: pk,
            lastHash: last_hash.to_owned()
        },
    };

    let msg = serde_json::to_string(&msg).unwrap();

    let req = client
        .post(&addr)
        .header("X-Loki-ephemkey", "86400")
        // .header("x-loki-long-poll", "")
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
        }
        Err(e) => {
            error!("Error requesting messages: {}", e);
            return vec![];
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct SnodesParams {
    pubKey: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetSnodesBody {
    method: String,
    params: SnodesParams
}

#[allow(dead_code)]
pub fn get_snodes_for_pk(sm: &SwarmManager, pk_str: &str) {

    let pk = PubKey::new(&pk_str).unwrap();

    let swarm_idx = sm.get_swarm_by_pk(&pk);

    let sn = &sm.swarms[swarm_idx as usize].nodes[0];

    let target = "/storage_rpc/v1";

    let addr = "https://localhost:".to_owned() + &sn.port + target;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build().unwrap();

    let msg = GetSnodesBody {
        method: "get_snodes_for_pubkey".to_owned(),
        params: SnodesParams {
            pubKey: pk_str.to_owned(),
        },
    };

    let msg = serde_json::to_string(&msg).unwrap();

    let req = client
        .post(&addr)
        .header("X-Loki-timestamp", "1540860811000")
        .body(msg);

    match req.send() {

        Ok(mut res) => {
            let mut body = String::new();
            res.read_to_string(&mut body).unwrap();
            dbg!(body);
        },
        Err(e) => {
            error!("Error requesting snode list: {}", e);
        }

    }

}

pub fn send_random_message_to_pk(sm: &SwarmManager, pk_str: &str, rng : &mut StdRng) -> Result<String, ()> {

    let num = rng.gen::<u64>();
    let msg = num.to_string() + &num.to_string() + &num.to_string();

    let res = send_message_to_pk(&sm, &pk_str, &msg);

    res.map(|()| msg)
}

pub fn send_message_to_pk(sm: &SwarmManager, pk_str: &str, msg: &str) -> Result<(), ()> {
    let pk = PubKey::new(&pk_str).unwrap();

    let swarm_idx = sm.get_swarm_by_pk(&pk);

    let sn = &sm.swarms[swarm_idx as usize].nodes[0];

    let res = send_message(&sn.port, &pk_str, &msg);

    if res.is_ok() {
        debug!("sent msg <{}> to sn {} (swarm {}, pk {})", &msg, &sn.port, &sm.swarms[swarm_idx as usize].swarm_id, &pk_str);
    } else {
        error!("could not send msg <{}> to sn {} (swarm {}, pk {})", &msg, &sn.port, &sm.swarms[swarm_idx as usize].swarm_id, &pk_str);
    }

    res
}

#[allow(dead_code)]
pub fn barrage_messages(port: &str) {

    let mut futs = vec![];

    for _ in 0..1000 {

        let client = hyper::Client::new();

        let uri = format!("https://0.0.0.0:{}/swarms/push/v1", port);

        let req = hyper::Request::builder().method("post").uri(uri).body(Body::from("hello")).unwrap();

        let fut = client.request(req).map(|_|{
            println!("It's a success!");
        }).map_err(|_err| {

        });

        futs.push(fut);
    }

    rt::run(futures::future::join_all(futs).map(|_| {}) );

}

pub fn send_random_message(sm: &SwarmManager, mut rng : &mut StdRng) -> Result<(String, String), ()> {
    // generate random PK
    // For now, PK is a random 256 bit string

    let pk = PubKey::gen_random(&mut rng);

    let pk_str = pk.to_string();

    let swarm_idx = sm.get_swarm_by_pk(&pk);

    // Note: commented out, as we want to make sure we send messages to a snode
    // that is online (for testing disconnected snodes in some tests)
    // let sn = &sm.swarms[swarm_idx as usize].nodes.choose(&mut rng).unwrap();
    let sn = &sm.swarms[swarm_idx as usize].nodes[0];

    let num = rng.gen::<u64>();
    let msg = num.to_string() + &num.to_string() + &num.to_string();

    let res = send_message(&sn.port, &pk_str, &msg);

    if res.is_ok() {
        info!("sending random message <{}> to {} to sn {} from swarm {}", msg, pk_str, &sn.port, swarm_idx);
        return Ok((pk_str.to_owned(), msg.to_owned()));
    }

    Err(())
}

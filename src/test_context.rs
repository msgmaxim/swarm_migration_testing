use crate::blockchain::{Blockchain, KeyPair, X25519KeyPair, Ed25519KeyPair};

use rand::prelude::*;
use std::collections::HashMap;

use std::fmt::{self, Debug, Display};
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

use crate::client::MessageResponse;

use crate::service_node::ServiceNode;
use crate::swarms::{PubKey, SpawnStrategy, Swarm};

use crate::client;

pub struct TestContext {
    bc: Arc<Mutex<Blockchain>>,
    messages: HashMap<String, Vec<String>>,
    latest_port: u16,
    bad_snodes: Vec<ServiceNode>,
    keypair_pool: Vec<(KeyPair, Ed25519KeyPair, X25519KeyPair)>,
    lokid_ports: Vec<u16>,
    rng: StdRng,
}

fn is_port_available(port: u16) -> bool {
    match std::net::TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

impl Display for TestContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.bc.lock().unwrap())
    }
}

impl Debug for TestContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self.bc.lock().unwrap())
    }
}

impl TestContext {
    fn read_keys() -> Vec<(KeyPair, Ed25519KeyPair, X25519KeyPair)> {
        let mut contents = String::new();
        // each row in keys.txt is: legacy sk | legacy pk | ed pk | curve sc | curve pk
        let mut key_file = std::fs::File::open("keys.txt").expect("could not open key file");
        key_file.read_to_string(&mut contents).unwrap();

        let keypair_pool: Vec<(KeyPair, Ed25519KeyPair, X25519KeyPair)> = contents
            .lines()
            .map(|pair| {
                let mut keys = pair.split_whitespace();

                let legacy = KeyPair {
                    seckey: keys.next().unwrap().to_owned(),
                    pubkey: keys.next().unwrap().to_owned(),
                };

                let ed_keys = Ed25519KeyPair {
                    seckey: keys.next().unwrap().to_owned(),
                    pubkey: keys.next().unwrap().to_owned(),
                };

                let x_keys = X25519KeyPair {
                    seckey: keys.next().unwrap().to_owned(),
                    pubkey: keys.next().unwrap().to_owned(),
                };

                (legacy, ed_keys, x_keys)
            })
            .collect();

        keypair_pool
    }

    pub fn new(bc: Arc<Mutex<Blockchain>>, lokid_ports: &[u16]) -> TestContext {

        let keypair_pool = TestContext::read_keys();

        println!("total keys: {}", keypair_pool.len());

        // TODO: poll earch SN every 200ms or so to see if they are alive
        // NOTE: THIS IS MISLEADING! The nodes will often be reported as dead
        // because they haven't had a chance to start their servers yet.

        // let bc_clone = bc.clone();
        // std::thread::spawn(move || {

        //     loop {

        //         std::thread::sleep(std::time::Duration::from_millis(1000));

        //         let bc = bc_clone.lock().unwrap();
                
        //         let swarms = bc.get_swarms();
        //         for s in swarms {
        //             for node in s.nodes {

        //                 if client::check_status(&node).is_err() {
        //                     eprintln!("[!] No response from node: {}", node.port);
        //                 }
        //             }
        //         }

        //     }

        // });

        TestContext {
            bc,
            messages: HashMap::new(),
            latest_port: 5901,
            bad_snodes: vec![],
            keypair_pool,
            lokid_ports: lokid_ports.to_owned(),
            rng: StdRng::seed_from_u64(0),
        }
    }

    pub fn snode_count(&self) -> usize {
        let sm = &self.bc.lock().unwrap().swarm_manager;

        let mut n = 0;
        for swarm in &sm.swarms {
            n += swarm.nodes.len();
        }
        n
    }

    pub fn send_message(&mut self, pk: &str, msg: &str) {
        if client::send_message_to_pk(&self.bc.lock().unwrap().swarm_manager, pk, msg).is_ok() {
            self.messages
                .entry(pk.to_owned())
                .or_insert(vec![])
                .push(msg.to_owned());
        }
    }

    /// Get messages for `pk` that come after the message with the specified `last_hash`
    pub fn get_new_messages(&self, pk: &PubKey, last_hash: &str) -> Vec<MessageResponse> {
        // 1. Find the closest swarm
        let sm = &self.bc.lock().unwrap().swarm_manager;
        let swarm_idx = sm.get_swarm_by_pk(&pk) as usize;
        let swarm = &sm.swarms[swarm_idx];

        // 2. Request messages from one of the nodes

        let sn = &swarm.nodes[0];

        client::request_messages_given_hash(&sn, &pk.to_string(), &last_hash)
    }

    pub fn send_random_message(&mut self) {
        if let Ok(msg) =
            client::send_random_message(&self.bc.lock().unwrap().swarm_manager, &mut self.rng)
        {
            self.messages.entry(msg.0).or_insert(vec![]).push(msg.1);
        }
    }

    pub fn send_random_message_to_pk(&mut self, pk: &str) {
        if let Ok(msg) = client::send_random_message_to_pk(
            &self.bc.lock().unwrap().swarm_manager,
            &pk,
            &mut self.rng,
        ) {
            self.messages
                .entry(pk.to_owned())
                .or_insert(vec![])
                .push(msg);
        }
    }

    /// Check that all previously sent messages are still available
    pub fn check_messages(&self) {
        let mut lost_count = 0;
        let mut messages_tested = 0;

        for key in self.messages.keys() {
            info!("checking messages for PK: {}", key);

            let expect_msgs = &self.messages[key];

            let sm = &self.bc.lock().unwrap().swarm_manager;

            let swarm_idx = sm.get_swarm_by_pk(&PubKey::new(&key).unwrap());
            let swarm = &sm.swarms[swarm_idx as usize];

            for sn in &swarm.nodes {
                if self.bad_snodes.contains(&sn) {
                    continue;
                };
                info!(
                    "requesting messages from: {} [{}]",
                    &swarm.swarm_id, &sn.port
                );

                let got_msgs = client::request_messages(&sn, key);
                let got_msgs: Vec<String> = got_msgs.iter().map(|x| x.data.clone()).collect();

                if expect_msgs.len() != got_msgs.len() {
                    warn!(
                        "wrong number of messages for pk: {}, exepcted {}, got {}",
                        key,
                        expect_msgs.len(),
                        got_msgs.len()
                    );
                }

                for msg in expect_msgs {
                    messages_tested += 1;

                    if !got_msgs.contains(&msg) {
                        warn!("message lost: {}", &msg);
                        warn!("only got: {:?}", &got_msgs);
                        lost_count += 1;
                    };
                }
            }
        }

        if lost_count == 0 {
            println!(
                "Test passed! ({}/{} messages)",
                messages_tested - lost_count,
                messages_tested
            );
        } else {
            println!("Messages lost: {}/{}", lost_count, messages_tested);
        }
    }

    pub fn print_stats(&self) {
        println!(
            "Total dissoved: {}",
            &self.bc.lock().unwrap().swarm_manager.stats.dissolved
        );
    }

    fn pop_keypair(&mut self) -> (KeyPair, Ed25519KeyPair, X25519KeyPair) {
        self.keypair_pool.pop().expect("Could not pop a key pair")
    }

    /// Swarm manager should decide where to push this SN
    fn add_snode_with_options(&mut self, spawn: SpawnStrategy) -> Option<ServiceNode> {
        let mut res = None;

        // find available port starting with 5904
        for i in (self.latest_port + 1)..7000 {
            if is_port_available(i) {
                // let keypair = self.bc.lock().unwrap();
                let port = i.to_string();

                let (legacy, ed_keys, x_keys) = self.pop_keypair();

                let lokid_port = self.lokid_ports.choose(&mut self.rng).unwrap();

                let sn = ServiceNode::new(
                    port,
                    legacy,
                    ed_keys,
                    x_keys,
                    *lokid_port,
                );

                self.bc.lock().unwrap().swarm_manager.add_snode(&sn, spawn);

                res = Some(sn);

                self.latest_port = i;

                break;
            }
        }

        res
    }

    pub fn add_snode(&mut self) {
        let _sn = self.add_snode_with_options(SpawnStrategy::Now);
    }

    /// Register a new SN, but spawn its server instance
    /// only after the specified period of time
    pub fn add_snode_delayed(&mut self, delay_ms: u64) {
        let sn = self.add_snode_with_options(SpawnStrategy::Later);

        let bc_copy = self.bc.clone();

        let _ = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            bc_copy
                .lock()
                .unwrap()
                .swarm_manager
                .restore_snode(&sn.expect("spawned node is none"));
        });
    }

    /// Shut down the snode's server without explicitly deregistering it.
    /// Mark this snode as known to have a problem, so that not being able to
    /// recieve messages from this node is not a problem
    pub fn disconnect_snode(&mut self) {
        let sn = self.bc.lock().unwrap().swarm_manager.disconnect_snode();
        self.bad_snodes.push(sn);
    }

    pub fn dissolve_swarm(&mut self, swarm_idx: usize) {
        self.bc
            .lock()
            .unwrap()
            .swarm_manager
            .dissolve_swarm(swarm_idx);
    }

    pub fn restart_snode(&mut self, delay_ms: u64) {
        let sn = self.bc.lock().unwrap().swarm_manager.disconnect_snode();

        println!("restarted snode: {}", sn.port);
        info!("restarted snode: {}", sn.port);

        // connect again after a short time

        let bc_copy = self.bc.clone();

        let _ = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));

            // TODO: need to check if the node has been dropped already
            // with tests, so there is no need to restore the node
            bc_copy.lock().unwrap().swarm_manager.restore_snode(&sn);
        });
    }

    pub fn drop_snode(&mut self) {
        self.bc.lock().unwrap().swarm_manager.drop_snode();
    }

    pub fn add_swarm<'a>(&mut self, n: usize) {
        let mut node_data: Vec<(u16, KeyPair, Ed25519KeyPair, X25519KeyPair)> = vec![];

        for i in (self.latest_port + 1)..7000 {
            if is_port_available(i) {
                let (legacy_keys, ed_keys, x_keys) = self.pop_keypair();

                node_data.push((i, legacy_keys, ed_keys, x_keys));
                if node_data.len() >= n {
                    self.latest_port = i;
                    break;
                }
            }
        }

        let mut bc = self.bc.lock().unwrap();
        bc.swarm_manager.add_swarm(node_data, &self.lokid_ports);
    }

    // TODO: ensure that we call this atomically with corresponding
    // swarm changes
    pub fn inc_block_height(&mut self) {
        self.bc.lock().unwrap().inc_block_height();
    }

    pub fn get_swarms(&self) -> Vec<Swarm> {
        self.bc.lock().unwrap().get_swarms()
    }
}

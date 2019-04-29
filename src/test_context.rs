use crate::rpc_server::Blockchain;

use rand::prelude::*;
use std::collections::HashMap;

use std::fmt::{self, Debug, Display};
use std::sync::{Arc, Mutex};

use crate::swarms::PubKey;
use crate::swarms::ServiceNode;
use crate::swarms::SpawnStrategy;

use crate::client::MessageResponse;

use crate::client;

pub struct TestContext {
    bc: Arc<Mutex<Blockchain>>,
    messages: HashMap<String, Vec<String>>,
    latest_port: u16,
    bad_snodes: Vec<ServiceNode>,
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
    pub fn new(bc: Arc<Mutex<Blockchain>>) -> TestContext {
        TestContext {
            bc,
            messages: HashMap::new(),
            latest_port: 5901,
            bad_snodes: vec![],
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
                warn!("requesting messages from: {} [{}]", &swarm.swarm_id, &sn.ip);

                let got_msgs = client::request_messages(&sn, key);
                let got_msgs: Vec<String> = got_msgs.iter().map(|x| x.data.clone()).collect();

                if expect_msgs.len() != got_msgs.len() {
                    error!(
                        "wrong number of messages for pk: {}, exepcted {}, got {}",
                        key,
                        expect_msgs.len(),
                        got_msgs.len()
                    );
                }

                for msg in expect_msgs {
                    messages_tested += 1;

                    if !got_msgs.contains(&msg) {
                        error!("message lost: {}", &msg);
                        error!("only got: {:?}", &got_msgs);
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

    /// Swarm manager should decide where to push this SN
    fn add_snode_with_options(&mut self, spawn: SpawnStrategy) -> Option<ServiceNode> {
        let mut res = None;

        // find available port starting with 5904
        for i in (self.latest_port + 1)..7000 {
            if is_port_available(i) {
                let sn = ServiceNode { ip: i.to_string() };
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

        let t = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            bc_copy.lock().unwrap().swarm_manager.restore_snode(&sn.expect("spawned node is none"));
        });
    }

    /// Shut down the snode's server without explicitly deregistering it.
    /// Mark this snode as known to have a problem, so that not being able to
    /// recieve messages from this node is not a problem
    pub fn disconnect_snode(&mut self) {
        let sn = self.bc.lock().unwrap().swarm_manager.disconnect_snode();
        self.bad_snodes.push(sn);
    }

    pub fn restart_snode(&mut self, delay_ms: u64) {
        let sn = self.bc.lock().unwrap().swarm_manager.disconnect_snode();

        println!("restarted snode: {}", sn.ip);
        info!("restarted snode: {}", sn.ip);

        // connect again after a short time

        let bc_copy = self.bc.clone();

        let t = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            bc_copy.lock().unwrap().swarm_manager.restore_snode(&sn);
        });
    }

    pub fn drop_snode(&mut self) {
        self.bc.lock().unwrap().swarm_manager.drop_snode();
    }

    pub fn add_swarm<'a>(&mut self, n: usize) {
        let mut ports: Vec<u16> = vec![];

        for i in (self.latest_port + 1)..7000 {
            if is_port_available(i) {
                ports.push(i);
                if ports.len() >= n {
                    self.latest_port = i;
                    break;
                }
            }
        }

        &self.bc.lock().unwrap().swarm_manager.add_swarm(&ports);
    }
}

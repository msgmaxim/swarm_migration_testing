use crate::client::MessagePair;
use crate::rpc_server::Blockchain;

use std::collections::HashMap;

use std::sync::{Arc, Mutex};

use crate::client;

pub struct TestContext {
    bc: Arc<Mutex<Blockchain>>,
    messages: HashMap<String, Vec<String>>,
}

impl TestContext {
    pub fn new(bc: Arc<Mutex<Blockchain>>) -> TestContext {
        TestContext {
            bc,
            messages: HashMap::new(),
        }
    }

    pub fn send_message(&mut self, pk: &str, msg: &str) {
        client::send_message_to_pk(&self.bc.lock().unwrap().swarm_manager, pk, msg);

        self.messages.entry(pk.to_owned()).or_insert(vec![]).push(msg.to_owned());
    }

    pub fn check_messages(&self) {

        let mut error_count = 0;

        for key in self.messages.keys() {
            println!("checking messages for PK: {}", key);

            let expect_msgs = &self.messages[key];

            let got_msgs = client::request_messages(&self.bc.lock().unwrap().swarm_manager, key);
            let got_msgs : Vec<String> = got_msgs.iter().map(|x| x.data.clone()).collect();

            if expect_msgs.len() != got_msgs.len() {
                error!("wrong number of messages for pk: {}, exepcted {}, got {}", key, expect_msgs.len(), got_msgs.len());
                error_count += 1;
            }

            for msg in expect_msgs {

                if !got_msgs.contains(&msg) {
                    error!("cannot find message {}", &msg);
                    error_count += 1;
                };
            }

        }
        if error_count == 0 {
            println!("Message test: ALL GOOD!");
        } else {
            println!("Error count: {}", error_count);
        }
    }

    /// Swarm manager should decide where to push this SN
    pub fn add_snode(&mut self, sn : &str) {

        &self.bc.lock().unwrap().swarm_manager.add_snode(&sn);

    }
}

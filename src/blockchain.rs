

use crate::swarms::*;
use rand::prelude::*;

use std::fmt::{self, Debug, Display};
use std::io::prelude::*;

fn gen_random_hash() -> String {
    let n1 = rand::thread_rng().gen::<u64>();
    let n2 = rand::thread_rng().gen::<u64>();
    let n3 = rand::thread_rng().gen::<u64>();
    let n4 = rand::thread_rng().gen::<u64>();

    format!("{:016x}{:016x}{:016x}{:016x}", n1, n2, n3, n4)
}

pub struct KeyPair {
    pub pubkey: String,
    pub seckey: String,
}

pub struct Blockchain {
    pub swarm_manager: SwarmManager,
    height: u64,
    keypair_pool: Vec<KeyPair>,
    block_hash: String,
    sys_time: std::time::SystemTime,
}

impl Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for swarm in &self.swarm_manager.swarms {
            if let Err(e) = write!(f, "[{}] ", swarm.nodes.len()) {
                return Err(e);
            }
        }

        Ok(())
    }
}

impl Debug for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for swarm in &self.swarm_manager.swarms {
            if let Err(e) = write!(f, "[{:?}] ", swarm) {
                return Err(e);
            }
        }

        Ok(())
    }
}

impl Blockchain {
    pub fn new(swarm_manager: SwarmManager) -> Blockchain {
        // 0 is used to indicate that SN haven't synced yet
        let height = 2;
        let block_hash = gen_random_hash();

        // read keys file
        let mut contents = String::new();
        let mut key_file = std::fs::File::open("keys.txt").expect("could not open key file");
        key_file.read_to_string(&mut contents).unwrap();
        println!("total keys: {}", contents.lines().count());
        let keypair_pool: Vec<KeyPair> = contents
            .lines()
            .map(|pair| {
                let mut pair = pair.split_whitespace();
                KeyPair {
                    seckey: pair.next().unwrap().to_owned(),
                    pubkey: pair.next().unwrap().to_owned(),
                }
            })
            .collect();

        Blockchain {
            swarm_manager,
            keypair_pool,
            height,
            block_hash,
            sys_time: std::time::SystemTime::now(),
        }
    }

    pub fn pop_keypair(&mut self) -> KeyPair {
        self.keypair_pool.pop().expect("Could not pop a key pair")
    }

    pub fn reset(&mut self) {
        self.swarm_manager.reset();
    }

    pub fn get_target_height(&self) -> u64 {

        if self.sys_time.elapsed().unwrap() >= std::time::Duration::from_secs(0) {
            self.height
        } else {
            0
        }
    }

    pub fn get_height(&self) -> u64 {
        self.height
    }

    pub fn get_block_hash(&self) -> &String {
        &self.block_hash
    }

    pub fn inc_block_height(&mut self) {
        self.height += 1;
        self.block_hash = gen_random_hash();
    }
}

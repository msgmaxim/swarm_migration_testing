
use crate::swarms::Swarm;
use crate::blockchain::Blockchain;
use std::sync::{Arc, Mutex};

pub trait BlockchainViewable {

    fn get_swarms(&self) -> Vec<Swarm>;

    fn get_height(&self) -> u64;

    fn get_block_hash(&self) -> String;

    fn get_target_height(&self) -> u64;

    fn get_hf(&self) -> u8;

}

#[derive(Debug)]
struct BlockchainData {
    swarms: Vec<Swarm>,
    height: u64,
    block_hash: String,
    target_height: u64,
}

#[derive(Debug)]
pub struct BlockchainView {
    cache : Arc<Mutex<BlockchainData>>,
}

impl Drop for BlockchainView {

    fn drop(&mut self) {
        println!("dropping BlockchainView");
    }
}

impl BlockchainView {
    pub fn new(bc: &Arc<Mutex<Blockchain>>, update_period: std::time::Duration) -> BlockchainView {
        let cache = BlockchainData { swarms : vec![], height : 0, block_hash : String::new(), target_height: 0};
        let cache = Arc::new(Mutex::new(cache));

        let bc = bc.clone();

        let cache_clone = cache.clone();
        std::thread::spawn(move || {

            loop {

                let bc = bc.lock().unwrap();
                let mut cache = cache_clone.lock().unwrap();

                let hash = bc.get_block_hash().clone();

                if hash != cache.block_hash {

                    cache.block_hash = hash;
                    cache.swarms = bc.get_swarms();
                    cache.height = bc.get_height();
                    cache.target_height = bc.get_target_height();

                }

                drop(cache);
                drop(bc);

                std::thread::sleep(update_period);
            }


        });

        BlockchainView { cache }
    }
}

impl BlockchainViewable for BlockchainView {

    fn get_swarms(&self) -> Vec<Swarm> {
        self.cache.lock().unwrap().swarms.clone()
    }

    fn get_height(&self) -> u64 {
        self.cache.lock().unwrap().height
    }

    fn get_block_hash(&self) -> String {
        self.cache.lock().unwrap().block_hash.clone()
    }

    fn get_target_height(&self) -> u64 {
        self.cache.lock().unwrap().target_height
    }

    fn get_hf(&self) -> u8 {
        // we start with height 20...
        15
        // if self.cache.lock().unwrap().height < 23 { 14 } else { 15 }
    }

}

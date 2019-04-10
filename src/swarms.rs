use rand::prelude::*;
use rand::seq::SliceRandom;
use std::fmt::{self, Debug};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ServiceNode {
    pub ip: String,
}

#[derive(Serialize, Clone)]
pub struct Swarm {
    pub swarm_id: u64,
    pub nodes: Vec<ServiceNode>,
}

impl Debug for Swarm {

    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: ", &self.swarm_id);
        for sn in &self.nodes {
            write!(f, "{:?} ", sn.ip);
        }
        Ok(())
    }

}

pub struct Stats {
    pub dissolved: u64
}

pub struct SwarmManager {
    pub swarms: Vec<Swarm>,
    pub children: Vec<std::process::Child>,
    pub stats : Stats,
    rng : StdRng,
}

// pub type PubKey = [u64; 4];
#[derive(Clone)]
pub struct PubKey {
    data: [u64; 4],
}

impl Debug for PubKey {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PubKey: <{}>", self.to_string())
    }

}

const MIN_SWARM_SIZE : usize = 3;

impl PubKey {
    pub fn new(data: &str) -> Option<PubKey> {
        if data.len() != 64 {
            return None;
        }

        let pk0 = u64::from_str_radix(&data[0..16], 16).unwrap();
        let pk1 = u64::from_str_radix(&data[16..32], 16).unwrap();
        let pk2 = u64::from_str_radix(&data[32..48], 16).unwrap();
        let pk3 = u64::from_str_radix(&data[48..64], 16).unwrap();

        Some(PubKey {
            data: [pk0, pk1, pk2, pk3],
        })
    }

    pub fn gen_random(rng: &mut StdRng) -> PubKey {

        let mut pk = [rng.next_u64(), rng.next_u64(), rng.next_u64(), rng.next_u64()];

        PubKey { data : pk }

    }

    pub fn to_string(&self) -> String {

        format!("{:016x}{:016x}{:016x}{:016x}", self.data[0], self.data[1], self.data[2], self.data[3])
    }
}

static SERVER_PATH: &'static str = "/Users/maxim/Work/loki-storage-server/build/httpserver/httpserver";

pub fn spawn_service_node(sn: &ServiceNode) -> Option<std::process::Child> {
    let mut server_process = std::process::Command::new(&SERVER_PATH);

    let path = std::path::Path::new("playground");
    if !path.exists() {
        std::fs::create_dir(&path).unwrap();
    }

    let path = path.join(&sn.ip);

    let path = std::path::Path::new(&path);

    if !path.exists() {
        std::fs::create_dir(&path).unwrap();
    }

    server_process.current_dir(&path);

    {
        let err_path = path.join("stderr.txt");

        let stderr_file = std::fs::File::create(&err_path).unwrap();

        let stdout_file = stderr_file.try_clone().unwrap();

        server_process.stderr(std::process::Stdio::from(stderr_file));
        server_process.stdout(std::process::Stdio::from(stdout_file));
    }

    server_process.arg("0.0.0.0");
    server_process.arg(sn.ip.to_string());
    server_process.arg("--log-level");
    server_process.arg("debug");

    match server_process.spawn() {
        Ok(child) => Some(child),
        Err(e) => {
            eprintln!("error spawning process: {}", e);
            None
        }
    }
}

impl SwarmManager {
    pub fn new() -> SwarmManager {
        SwarmManager {
            swarms: vec![],
            children: vec![],
            stats: Stats { dissolved: 0 },
            rng : StdRng::seed_from_u64(1)
        }
    }

    pub fn reset(&mut self) {

        for swarm in &self.swarms {
            for sn in &swarm.nodes {
                crate::send_req_to_quit(&sn);
            }
        }

        self.swarms.clear();
        self.children.clear();
        self.stats.dissolved = 0;
        self.rng = StdRng::seed_from_u64(1);

    }

    pub fn add_swarm<'a>(&mut self, nodes: &[u16]) {
        let swarm_id = self.get_next_swarm_id();

        warn!("using {} as swarm id", swarm_id);

        let nodes: Vec<ServiceNode> = nodes
            .iter()
            .map(|ip| ServiceNode { ip: ip.to_string() })
            .collect();

        for node in &nodes {
            if let Some(child) = spawn_service_node(&node) {
                warn!("NEW SNODE: {}", &node.ip);
                self.children.push(child);
            } else {
                error!("Could not spawn node!");
            }
        }

        let swarm = Swarm { swarm_id, nodes };

        self.swarms.push(swarm);
    }

    pub fn dissolve_swarm(&mut self, idx : usize) {

        warn!("dissolving swarm: {}", self.swarms[idx].swarm_id);

        if self.swarms.len() == 1 {
            error!("Would dissolve the last swarm. Keeping it alive instead.");
            return;
        }

        self.stats.dissolved += 1;

        let swarm = self.swarms.remove(idx);

        for node in swarm.nodes {
            let target = self.swarms.choose_mut(&mut self.rng).unwrap();
            target.nodes.push(node);
        }

    }

    /// get index into swarms by client's public key
    pub fn get_swarm_by_pk(&self, pk: &PubKey) -> usize {
        let pk = pk.data;
        let res = pk[0] ^ pk[1] ^ pk[2] ^ pk[3];

        const MAX_VALUE : u64 = std::u64::MAX - 1;

        let (mut cur_best, mut min_dist) = (0, std::u64::MAX);
        let mut leftmost = std::u64::MAX;
        let mut leftmost_idx = 0;
        let mut rightmost = 0;
        let mut rightmost_idx = 0;

        for (idx, sw) in self.swarms.iter().enumerate() {
            let dist = if sw.swarm_id > res {
                sw.swarm_id - res
            } else {
                res - sw.swarm_id
            };
            if dist < min_dist {
                min_dist = dist;
                cur_best = idx;
            }

            if sw.swarm_id < leftmost {
                leftmost = sw.swarm_id;
                leftmost_idx = idx;
            }

            if sw.swarm_id > rightmost {
                rightmost = sw.swarm_id;
                rightmost_idx = idx;
            }
        }

        if res > rightmost {

            let dist = (MAX_VALUE - res) + leftmost;
            if dist < min_dist {
                cur_best = leftmost_idx;
            }

        } else if res < leftmost {

            let dist = res + (MAX_VALUE - rightmost);
            if dist < min_dist {
                cur_best = rightmost_idx;
            }

        }

        cur_best
    }

    // This seems to work fine, but I could add some unit tests
    pub fn get_next_swarm_id(&self) -> u64 {
        let mut ids: Vec<u64> = self.swarms.iter().map(|swarm| swarm.swarm_id).collect();

        if ids.len() == 0 {
            return 0;
        } else if ids.len() == 1 {
            return std::u64::MAX / 2;
        }

        ids.sort();

        let distances: Vec<(usize, u64)> = ids
            .iter()
            .enumerate()
            .zip(ids.iter().skip(1).chain([std::u64::MAX].iter()))
            .map(|((idx, cur), next)| (idx, next - cur))
            .collect();

        let (left_idx, max_dist) = distances.iter().max_by_key(|(_, dist)| dist).unwrap();

        let next_id = ids[*left_idx] + max_dist / 2;

        return next_id;
    }


    /// This does not modify swarm structure leaving the
    /// disconnected snode in the list.
    pub fn disconnect_snode(&mut self) -> ServiceNode {

        let mut swarm = &self.swarms.choose(&mut self.rng).unwrap();

        let snode = swarm.nodes.choose(&mut self.rng).unwrap();

        crate::send_req_to_quit(snode);

        warn!("disconnected snode: {}", snode.ip);

        snode.clone()
    }

    /// Drop one random snode
    pub fn drop_snode(&mut self) {

        let swarm_idx = self.rng.gen_range(0, self.swarms.len());
        let swarm = &mut self.swarms[swarm_idx];

        let node_idx = self.rng.gen_range(0, swarm.nodes.len());
        let node = swarm.nodes.remove(node_idx);

        warn!("dropping snode {} from swarm {}", &node.ip, &swarm.swarm_id);

        crate::send_req_to_quit(&node);

        // ==== Try to steal from existing swarms ====
        if swarm.nodes.len() >= MIN_SWARM_SIZE {
            return;
        }

        let mut big_swarms: Vec<Swarm> = self.swarms.iter().filter(|s| s.nodes.len() > MIN_SWARM_SIZE ).cloned().collect();

        info!("Have {} swarms to steal from", big_swarms.len());

        if big_swarms.len() > 0 {
            let mut big_swarm = big_swarms.choose_mut(&mut self.rng).unwrap();
            let node_idx = self.rng.gen_range(0, big_swarm.nodes.len());
            let mov_node = big_swarm.nodes.remove(node_idx);

            for swarm in &mut self.swarms {
                let mut del_idx : Option<usize> = None;
                if swarm.swarm_id != big_swarm.swarm_id {
                    continue;
                }

                for (idx, node) in &mut swarm.nodes.iter().enumerate() {
                    if node.ip == mov_node.ip {
                        del_idx = Some(idx);
                        break;
                    }
                }

                if let Some(del_idx) = del_idx {
                    assert_eq!(swarm.nodes[del_idx].ip, mov_node.ip);
                    swarm.nodes.remove(del_idx);

                    break;
                }
            }

            let swarm = &mut self.swarms[swarm_idx];
            warn!("moved snode {} from swarm {} to {}", &node.ip, &big_swarm.swarm_id, &swarm.swarm_id);
            swarm.nodes.push(mov_node);
        } else {
            // dissolve the swarm
            self.dissolve_swarm(swarm_idx);
        }
    }

    pub fn add_snode(&mut self, sn : &str) {

        let sn = ServiceNode { ip: sn.to_owned() };

        warn!("NEW SNODE: {}", &sn.ip);
        let child = spawn_service_node(&sn).expect("error spawning a service node");
        self.children.push(child);

        // Figure out which swarm this node is to join
        let mut rand_swarm = self.swarms.choose_mut(&mut self.rng).unwrap();

        info!("choosing swarm: {}", rand_swarm.swarm_id);

        rand_swarm.nodes.push(sn);

        // See if we need to make a new swarm
        let total_extra = self.swarms.iter().fold(0, |sum, x| if x.nodes.len() > MIN_SWARM_SIZE { sum + x.nodes.len() - MIN_SWARM_SIZE} else { sum } );

        info!("total extra: {}", total_extra);

        if total_extra > MIN_SWARM_SIZE {
            // create new swarm

            let mut nodes_to_move = vec![];

            while nodes_to_move.len() < 3 {

                let rand_swarm = self.swarms.choose_mut(&mut self.rng).unwrap();
                if rand_swarm.nodes.len() <= MIN_SWARM_SIZE { continue; }
                let idx = self.rng.gen_range(0, rand_swarm.nodes.len());

                let node = rand_swarm.nodes.remove(idx);
                nodes_to_move.push(node);
            }


            let swarm_id = self.get_next_swarm_id();
            let swarm = Swarm { swarm_id, nodes : nodes_to_move };
            self.swarms.push(swarm);
            warn!("using {} as swarm id", swarm_id);

        }

    }

    pub fn create_snode(&mut self, sn: ServiceNode, swarm_idx: usize) {
        warn!("NEW SNODE: {}", &sn.ip);
        let child = spawn_service_node(&sn).expect("error spawning a service node");
        self.children.push(child);

        // TODO: use loki rules to determine where the node should go
        self.swarms[swarm_idx].nodes.push(sn);
    }
}

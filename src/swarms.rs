use rand::RngCore;

#[derive(Serialize, Debug)]
pub struct ServiceNode {
    pub ip: String,
}

#[derive(Serialize, Debug)]
pub struct Swarm {
    pub swarm_id: u64,
    pub nodes: Vec<ServiceNode>,
}

pub struct SwarmManager {
    pub swarms: Vec<Swarm>,
    pub children: Vec<std::process::Child>,
}

// pub type PubKey = [u64; 4];

pub struct PubKey {
    data: [u64; 4],
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

    pub fn gen_random() -> PubKey {

        let mut rng = rand::thread_rng();

        let mut pk = [rng.next_u64(), rng.next_u64(), rng.next_u64(), rng.next_u64()];

        PubKey { data : pk }

    }

    pub fn to_string(&self) -> String {

        format!("{:x}{:x}{:x}{:x}", self.data[0], self.data[1], self.data[2], self.data[3])
    }
}

static SERVER_PATH: &'static str = "/Users/maxim/Work/loki-storage-server/build/httpserver";

pub fn spawn_service_node(sn: &ServiceNode) -> Option<std::process::Child> {
    let mut server_process = std::process::Command::new(&SERVER_PATH);

    let path = std::path::Path::new(&sn.ip);

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
        }
    }

    pub fn add_swarm<'a>(&mut self, nodes: &[&'a str]) {
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

        let mut swarm = self.swarms.remove(idx);

        // TODO: randomise this
        self.swarms[3].nodes.append(&mut swarm.nodes);
    }

    /// get index into swarms by client's public key
    pub fn get_swarm_by_pk(&self, pk: &PubKey) -> usize {
        let pk = pk.data;
        let res = pk[0] ^ pk[1] ^ pk[2] ^ pk[3];

        println!("res: {}", &res);

        let (mut min_idx, mut min_dist) = (0, std::u64::MAX);

        for (idx, sw) in self.swarms.iter().enumerate() {
            let dist = if sw.swarm_id > res {
                sw.swarm_id - res
            } else {
                res - sw.swarm_id
            };
            if dist < min_dist {
                min_dist = dist;
                min_idx = idx;
            }
        }

        // Note: we wrap 0 and std::u64::MAX, so handle this case separately,
        // but only if we are on the right side of the first swarm id (so we don't overflow)
        if res > self.swarms[0].swarm_id {

            let odd_dist = (std::u64::MAX - res + self.swarms[0].swarm_id);

            if odd_dist < min_dist {
                return 0;
            }
        }

        min_idx
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

    pub fn add_snode(&mut self, sn : &str) {


        

    }

    pub fn create_snode(&mut self, sn: ServiceNode, swarm_idx: usize) {
        warn!("NEW SNODE: {}", &sn.ip);
        let child = spawn_service_node(&sn).expect("error spawning a service node");
        self.children.push(child);

        // TODO: use loki rules to determine where the node should go
        self.swarms[swarm_idx].nodes.push(sn);
    }
}

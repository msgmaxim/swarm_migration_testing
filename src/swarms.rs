#[derive(Serialize, Clone)]
pub struct ServiceNode {
    pub ip: String,
}

#[derive(Serialize, Clone)]
pub struct Swarm {
    pub swarm_id: u64,
    pub nodes: Vec<ServiceNode>,
}

#[derive(Serialize, Clone)]
pub struct SwarmManager {
    pub swarms: Vec<Swarm>,
}

impl SwarmManager {


    // pub fn add_swarm<'a>(&mut self, nodes : Vec<&'a str> ) {
    pub fn add_swarm<'a>(&mut self, nodes : &[&'a str]) {

        let swarm_id = self.get_next_swarm_id();

        warn!("using {} as swarm id", swarm_id);

        let nodes : Vec<ServiceNode> = nodes.iter().map(|ip| ServiceNode{ip: ip.to_string() }).collect();

        let swarm = Swarm { swarm_id, nodes };

        self.swarms.push(swarm);
    }

    // This seems to work fine, but I could add some unit tests
    pub fn get_next_swarm_id(&self) -> u64 {

        // get all swarm ids
        let mut ids: Vec<u64> = self.swarms.iter().map(|swarm| swarm.swarm_id).collect();

        if ids.len() == 0 {
            return 0;
        } else if ids.len() == 1 {
            return std::u64::MAX / 2;
        }

        ids.sort();

        let distances : Vec<(usize, u64)> = ids.iter().enumerate().zip( ids.iter().skip(1).chain( [std::u64::MAX].iter() ) ).map(|((idx, cur), next)| {
            println!("{:?}", (&cur, &next));
            (idx, next - cur)
            // 1
        }).collect();

        let (left_idx, max_dist) = distances.iter().max_by_key(|(_, dist)| dist).unwrap();

        dbg!(&ids);

        dbg!(&distances);

        dbg!(&max_dist);

        println!("{:?}", left_idx);

        let next_id = ids[*left_idx] + max_dist / 2;

        return next_id;
    }
}

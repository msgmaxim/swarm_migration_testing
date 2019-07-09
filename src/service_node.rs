
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceNode {
    pub port: String,
    pub pubkey: String,
    pub seckey: String,
}

impl ServiceNode {
    pub fn new(port: String, pubkey: String, seckey: String) -> ServiceNode {
        ServiceNode {
            port,
            pubkey,
            seckey,
        }
    }
}

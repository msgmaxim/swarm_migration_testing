
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceNode {
    pub port: String,
    pub pubkey: String,
    pub seckey: String,
    pub lokid_port: u16,
}

impl ServiceNode {
    pub fn new(port: String, pubkey: String, seckey: String, lokid_port: u16) -> ServiceNode {
        ServiceNode {
            port,
            pubkey,
            seckey,
            lokid_port,
        }
    }
}

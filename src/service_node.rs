
use crate::blockchain::{KeyPair, X25519KeyPair, Ed25519KeyPair};

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceNode {
    pub port: String,
    pub pubkey: String,
    pub seckey: String,
    pub pubkey_x25519: String,
    pub seckey_x25519: String,
    pub ed_keys: Ed25519KeyPair,
    pub lokid_port: u16,
}

impl ServiceNode {
    pub fn new(port: String, keypair: KeyPair, ed_keys: Ed25519KeyPair, x_keys: X25519KeyPair, lokid_port: u16) -> ServiceNode {
        ServiceNode {
            port,
            pubkey: keypair.pubkey,
            seckey: keypair.seckey,
            pubkey_x25519: x_keys.pubkey,
            seckey_x25519: x_keys.seckey,
            ed_keys,
            lokid_port,
        }
    }
}

use std::collections::HashMap;
use crate::models::{NodeInfo, RingState};
use sha2::{Digest, Sha256};


#[derive(Clone, PartialEq, Debug)]
pub struct VirtualNode {
    pub position: u64,
    pub node_id: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct HashRing {
    pub virtual_nodes: Vec<VirtualNode>,
    pub virtual_nodes_per_physical: u32,
    pub node_map: HashMap<String, NodeInfo>,
}

impl HashRing {
    pub fn new() -> Self {
        HashRing {
            virtual_nodes: Vec::new(),
            virtual_nodes_per_physical: 150,
            node_map: HashMap::new(),
        }
    }
    fn hash(input: &str) -> u64{
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let bytes: [u8; 8] = result[..8]
            .try_into()
            .expect("Slice with incorrect length");

        u64::from_be_bytes(bytes)
    }
    pub fn add_node(&mut self, node_info: NodeInfo){
        self.node_map.insert(node_info.node_id.clone(), node_info.clone());
        for i in 0..self.virtual_nodes_per_physical {
            let virtual_node_id = format!("{}-{}", node_info.node_id, i);
            let position = Self::hash(&virtual_node_id);
            self.virtual_nodes.push(VirtualNode { position, node_id: node_info.node_id.clone() });
        }
        self.virtual_nodes
        .sort_by_key(|virtual_node| virtual_node.position);
    }
    pub fn remove_node(&mut self, node_id: &str){
        self.node_map.remove(node_id);
        self.virtual_nodes.retain(|vn| vn.node_id != node_id);
    }
    pub fn find_primary(&self, key: &str) -> Option<&NodeInfo> {
        if self.virtual_nodes.is_empty() {
            return None;
        }
        let hash_key = Self::hash(key);

        for virt_node in &self.virtual_nodes{
            if virt_node.position>=hash_key {
                return self.node_map.get(&virt_node.node_id);
            }
        }
        let first = &self.virtual_nodes[0];
        self.node_map.get(&first.node_id)
    }
    pub fn find_replicas(&self, key: &str) -> Vec<&NodeInfo> {
        let mut replicas = Vec::new();

        let primary = match self.find_primary(key) {
            Some(node) => node,
            None => return replicas,
        };

        let hash_key = Self::hash(key);

        let mut start_index = 0;

        for (i, vnode) in self.virtual_nodes.iter().enumerate() {
            if vnode.position >= hash_key {
                start_index = i;
                break;
            }
        }

        for i in 1..=self.virtual_nodes.len() {
            let index = (start_index + i) % self.virtual_nodes.len();

            let vnode = &self.virtual_nodes[index];

            if vnode.node_id != primary.node_id {
                if let Some(node) = self.node_map.get(&vnode.node_id) {
                    let already_exists = replicas
                        .iter()
                        .any(|n| n.node_id == node.node_id);

                    if !already_exists {
                        replicas.push(node);
                    }
                }
            }

            if replicas.len() == 2 {
                break;
            }
        }

        replicas
    }
    pub fn get_all_nodes(&self) -> Vec<NodeInfo>{
        let mut ans = Vec::new();
        for(_,node) in &self.node_map{
            ans.push(node.clone());
        }
        ans
    }
    pub fn to_ring_state(&self) -> RingState{
        let ans = self.get_all_nodes();
        RingState{
            nodes: ans,
            virtual_nodes: self
            .virtual_nodes
            .iter()
            .map(|vnode| (vnode.position, vnode.node_id.clone()))
            .collect(),

            replication_factor: 3,
        }
    }
    pub fn from_ring_state(ring_state: RingState) -> Self {
        let mut node_map = HashMap::new();

        for node in ring_state.nodes.clone() {
            node_map.insert(node.node_id.clone(), node);
        }

        let virtual_nodes = ring_state
            .virtual_nodes
            .into_iter()
            .map(|(position, node_id)| VirtualNode {
                position,
                node_id,
            })
            .collect();

        HashRing {
            virtual_nodes,
            node_map,
            virtual_nodes_per_physical: 150,
        }
    }
}
#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::{NodeInfo, NodeStatus};

    fn create_node(id: &str, port: u16) -> NodeInfo {
        NodeInfo {
            node_id: id.to_string(),
            node_port: port,
            host: "127.0.0.1".to_string(),
            status: NodeStatus::Alive,
            last_heartbeat: None,
        }
    }

    fn build_ring() -> HashRing {
        let mut ring = HashRing::new();

        ring.add_node(create_node("worker-1", 8081));
        ring.add_node(create_node("worker-2", 8082));
        ring.add_node(create_node("worker-3", 8083));
        ring.add_node(create_node("worker-4", 8084));

        ring
    }

    #[test]
    fn same_key_maps_to_same_primary() {
        let ring = build_ring();

        let first = ring
            .find_primary("my-key")
            .unwrap()
            .node_id
            .clone();

        for _ in 0..10 {
            let current = ring
                .find_primary("my-key")
                .unwrap()
                .node_id
                .clone();

            assert_eq!(first, current);
        }
    }

    #[test]
    fn replicas_are_not_primary() {
        let ring = build_ring();

        let primary = ring.find_primary("alpha").unwrap();

        let replicas = ring.find_replicas("alpha");

        for replica in replicas {
            assert_ne!(replica.node_id, primary.node_id);
        }
    }

    #[test]
    fn replicas_are_distinct() {
        let ring = build_ring();

        let replicas = ring.find_replicas("beta");

        assert_eq!(replicas.len(), 2);

        assert_ne!(replicas[0].node_id, replicas[1].node_id);
    }

    #[test]
    fn removing_node_changes_only_its_keys() {
        let mut ring = build_ring();

        let mut original: HashMap<String, String> = HashMap::new();
        for i in 0..100 {
            let key = format!("key-{}", i);

            let owner = ring
                .find_primary(&key)
                .unwrap()
                .node_id
                .clone();

            original.insert(key, owner);
        }

        ring.remove_node("worker-2");

        for (key, old_owner) in original {
            let new_owner = ring
                .find_primary(&key)
                .unwrap()
                .node_id
                .clone();

            if old_owner != "worker-2" {
                assert_eq!(old_owner, new_owner);
            }
        }
    }

    #[test]
    fn empty_ring_returns_none() {
        let ring = HashRing::new();

        let result = ring.find_primary("test-key");

        assert!(result.is_none());
    }
}
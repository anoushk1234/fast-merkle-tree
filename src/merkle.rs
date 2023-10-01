use rayon::prelude::*;
use solana_program::hash::{hash, hashv, Hash};
use thiserror::Error;
pub const DEFAULT_LEAF: [u8; 32] = [
    110, 52, 11, 156, 255, 179, 122, 152, 156, 165, 68, 230, 187, 120, 10, 44, 120, 144, 29, 63,
    179, 55, 56, 118, 133, 17, 163, 6, 23, 175, 160, 29,
];

pub const LEAF_PREFIX: &[u8] = &[0];
pub const NODE_PREFIX: &[u8] = &[1];

// hash_leaf and hash_node prepend a prefix 0x0 and 0x1 to prevent second pre-image attacks
// Refer: https://en.wikipedia.org/wiki/Merkle_tree#Second_preimage_attack
#[macro_export]
macro_rules! hash_leaf {
    ($leaf:ident) => {
        hashv(&[LEAF_PREFIX, $leaf.as_ref()])
    };
}
macro_rules! hash_node {
    ($lnode:ident,$rnode:ident) => {
        hashv(&[NODE_PREFIX, $lnode.as_ref(), $rnode.as_ref()])
    };
}

#[derive(Default, Debug)]
pub struct MerkleTree {
    pub leaf_count: usize,
    pub nodes: Vec<Hash>,
    pub current_leaf_index: usize,
}

impl MerkleTree {
    pub fn calculate_height(leaf_count: usize) -> usize {
        if leaf_count > 0 {
            fast_math::log2(leaf_count as f32).ceil() as usize
        } else {
            0
        }
    }
    pub fn calculate_next_level_len(current_level_len: usize) -> usize {
        if current_level_len > 1 {
            // (current_level_len as f64 / 2.0).ceil() as usize
            if current_level_len % 2 == 0 {
                current_level_len / 2
            } else {
                (current_level_len + 1) / 2
            }
        } else {
            0
        }
    }
    pub fn calculate_max_capacity(leaf_count: usize) -> usize {
        if leaf_count > 0 {
            let mut level_leaf_count = leaf_count as usize;
            let mut node_count = level_leaf_count;
            while level_leaf_count > 1 {
                // level_leaf_count = (level_leaf_count as f64 / 2.0).ceil() as usize;
                level_leaf_count = if level_leaf_count % 2 == 0 {
                    level_leaf_count / 2
                } else {
                    (level_leaf_count + 1) / 2
                };
                node_count += level_leaf_count;
                //
            }
            node_count
        } else {
            0
        }
    }
    pub fn new(leaf_count: usize) -> Self {
        let max_capacity = MerkleTree::calculate_max_capacity(leaf_count);
        let mut nodes = Vec::with_capacity(max_capacity);
        // for _ in 0..leaf_count {
        //     nodes.push(DEFAULT_LEAF.into());
        // }

        // (0..leaf_count)
        //     .into_par_iter()
        //     .for_each(|leaf_index| nodes[leaf_index] = DEFAULT_LEAF.into());
        nodes.par_extend(
            (0..leaf_count)
                .into_par_iter()
                .map(|_| Hash::from(DEFAULT_LEAF)),
        );
        Self {
            leaf_count,
            nodes,
            current_leaf_index: 0,
        }
    }

    pub fn insert<T: AsRef<[u8]>>(self: &mut Self, leaf: T) -> Option<&mut Self> {
        if self.current_leaf_index == self.leaf_count {
            return None;
        }

        let leaf_node = hash_leaf!(leaf);

        // println!(
        //     "current_leaf_index: {} leaf {}",
        //     self.current_leaf_index,
        //     String::from_utf8(leaf.as_ref().to_vec()).unwrap()
        // );
        if self.current_leaf_index == 0 {
            self.nodes[0] = leaf_node;
        } else {
            self.nodes[self.current_leaf_index] = leaf_node;
        }
        self.current_leaf_index += 1;
        Some(self)
    }
    pub fn get_value(self: &Self, leaf_index: usize) -> Option<&Hash> {
        self.nodes[0..self.leaf_count].get(leaf_index)
    }
    pub fn get_root(self: &mut Self) -> Option<&Hash> {
        let height = Self::calculate_height(self.leaf_count);
        let mut current_level: usize = height;

        let mut prev_level_len: usize = 0;
        let mut current_level_len: usize = self.leaf_count;

        let mut level_cache = Vec::with_capacity(current_level_len);

        let mut pairs = self.nodes.chunks(2);

        while current_level > 0 {
            let pair = pairs.next();
            match pair {
                Some([lnode, rnode]) => {
                    let inter_node = hash_node!(lnode, rnode);
                    level_cache.push(inter_node);
                }
                Some([lnode]) => {
                    let inter_node = hash_node!(lnode, lnode);
                    level_cache.push(inter_node);
                }
                _ => {
                    self.nodes.append(&mut level_cache);
                    current_level -= 1;

                    prev_level_len += current_level_len;
                    current_level_len = Self::calculate_next_level_len(current_level_len);
                    level_cache = Vec::with_capacity(current_level_len);
                    pairs = self.nodes[(prev_level_len)..(prev_level_len + current_level_len)]
                        .chunks(2);
                }
            }
            // if let Some([lnode, rnode]) = pair {
            //     let inter_node = hash_node!(lnode, rnode);
            //     level_cache.push(inter_node);
            // } else if let Some([lnode]) = pair {
            //     let inter_node = hash_node!(lnode, lnode);
            //     level_cache.push(inter_node);
            // } else {
            //     self.nodes.append(&mut level_cache);
            //     current_level -= 1;
            //
            //     prev_level_len += current_level_len;
            //     current_level_len = Self::calculate_next_level_len(current_level_len);
            //     level_cache = Vec::with_capacity(current_level_len);
            //     pairs =
            //         self.nodes[(prev_level_len)..(prev_level_len + current_level_len)].chunks(2);
            // }
        }
        self.nodes.iter().last()
    }
    pub fn get_opening(self: &Self, leaf_index: usize) -> Option<Vec<Hash>> {
        if leaf_index > self.leaf_count - 1 {
            return None;
        };
        let height = Self::calculate_height(self.leaf_count);
        let mut current_index = leaf_index;
        let mut current_level_len: usize = self.leaf_count;
        let mut current_level: usize = height + 1;
        let mut path: Vec<Hash> = vec![];

        let mut right_node = None;
        let mut left_node = None;
        let mut current_level_nodes = &self.nodes[0..self.leaf_count];
        let mut prev_level_len: usize = 0;
        while current_level > 0 {
            match left_node {
                Some(lnode) => path.push(lnode),
                _ => {}
            }
            match right_node {
                Some(rnode) => path.push(rnode),
                _ => {}
            }
            if current_index % 2 == 0 {
                if current_index < current_level_len - 1 {
                    right_node = Some(current_level_nodes[current_index + 1]);
                } else {
                    right_node = Some(current_level_nodes[current_index]);
                }
            } else {
                left_node = Some(current_level_nodes[current_index - 1]);
                right_node = None;
            }
            println!(
                "Level: {:?} Nodes: {:?} Index: {:?}",
                current_level, current_level_nodes, current_index
            );
            current_index /= 2;
            prev_level_len += current_level_len;
            current_level_len = Self::calculate_next_level_len(current_level_len);
            current_level -= 1;
            current_level_nodes = &self.nodes[prev_level_len..(prev_level_len + current_level_len)];
        }

        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fast_math;
    use hex;
    use rs_merkle::{algorithms::Sha256, Hasher, MerkleTree as RsTree};
    use solana_merkle_tree::MerkleTree as SMT;
    use solana_program::hash::{hashv, Hash};
    const TEST: &[&[u8]] = &[
        b"my", b"very", b"eager", b"mother", b"just", b"served", b"us", b"nine", b"pizzas",
        b"make", b"prime",
    ];
    const PROTO: &[&[u8]] = &[b"my", b"very", b"eager", b"mother"];
    #[test]
    fn tryit() {
        let mut k = MerkleTree::new(4);
        // println!("h2 {:?}", MerkleTree::calculate_max_capacity(10));
        // println!("h4 {:?}", MerkleTree::calculate_height(10));
        // println!("h4 {:?}", MerkleTree::calculate_height(8));
        // println!("h4 {:?}", MerkleTree::calculate_height(6));
        // println!("h4 {:?}", MerkleTree::calculate_height(4));
        // println!("h4 {:?}", MerkleTree::calculate_height(2));
        // println!("h4 {:?}", MerkleTree::calculate_height(1));

        for item in PROTO {
            let res = k.insert(item);
            // println!("res: {:?}", res.is_some());
        }
        let root = k.get_root();
        let opening = k.get_opening(3).unwrap();
        println!("MY_MERKLE_ROOT: {:?}", opening);
        // for (i, item) in k.nodes.iter().enumerate() {
        //     println!("INDEX: {:?} ITEM: {:?}", i, item);
        // }
        //
        // println!("TREE: {:?}", opening);
        // let mut verify_root = Hash::default();
        // for i in 0..opening.len() {
        //     if i !== 0{
        //     let lnode = opening[i].to_bytes();
        //     let rnode = opening[i + 1].to_bytes();
        //     verify_root = hash_node!(lnode, rnode);
        //             }
        //     else{
        //         verify_root = hash_leaf
        //     }
        //     }
        // println!("MY_PROOF: {:?}", verify_root);
        let mt = SMT::new(PROTO);

        println!("SOLANAS_MERKLE_ROOT: {:?}", mt.find_path(3));
    }
}

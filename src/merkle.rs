// use rayon::prelude::*;
use solana_program::hash::{hashv, Hash};
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
        // The hash function can be easily replace with any other
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
    /// Calculates the height of a tree with n leaves (n = 2^h).
    pub fn calculate_height(leaf_count: usize) -> usize {
        if leaf_count > 0 {
            fast_math::log2(leaf_count as f32).ceil() as usize
        } else {
            0
        }
    }
    /// Returns the length of array for the next level of the tree.
    pub fn calculate_next_level_len(current_level_len: usize) -> usize {
        if current_level_len > 1 {
            if current_level_len % 2 == 0 {
                current_level_len / 2
            } else {
                (current_level_len + 1) / 2
            }
        } else {
            0
        }
    }
    /// Returns the vector capacity required for a tree of given leaf count.
    pub fn calculate_max_capacity(leaf_count: usize) -> usize {
        if leaf_count > 0 {
            let mut level_leaf_count = leaf_count as usize;
            let mut node_count = level_leaf_count;
            while level_leaf_count > 1 {
                level_leaf_count = if level_leaf_count % 2 == 0 {
                    level_leaf_count / 2
                } else {
                    (level_leaf_count + 1) / 2
                };
                node_count += level_leaf_count;
            }
            node_count
        } else {
            0
        }
    }
    /// Construct a new instance of the Merkle Tree.
    pub fn new(leaf_count: usize) -> Self {
        let max_capacity = MerkleTree::calculate_max_capacity(leaf_count);
        let mut nodes = Vec::with_capacity(max_capacity);
        for _ in 0..leaf_count {
            nodes.push(DEFAULT_LEAF.into());
        }

        Self {
            leaf_count,
            nodes,
            current_leaf_index: 0,
        }
    }

    /// Inserts a single leaf into the tree.
    pub fn insert<T: AsRef<[u8]>>(self: &mut Self, leaf: T) -> Result<&mut Self, MerkleTreeError> {
        if self.current_leaf_index == self.leaf_count {
            return Err(MerkleTreeError::LeafIndexOutOfBounds(format!(
                "New leaf exceeds size of tree: {}",
                self.leaf_count,
            )));
        }

        let leaf_node = hash_leaf!(leaf);

        if self.current_leaf_index == 0 {
            self.nodes[0] = leaf_node;
        } else {
            self.nodes[self.current_leaf_index] = leaf_node;
        }
        self.current_leaf_index += 1;
        Ok(self)
    }

    /// Returns the leaf at given index.
    pub fn get_value(self: &Self, leaf_index: usize) -> Option<&Hash> {
        self.nodes[0..self.leaf_count].get(leaf_index)
    }

    /// Returns the Merkle Root of the tree.
    pub fn get_root(self: &mut Self) -> Option<&Hash> {
        let height = Self::calculate_height(self.leaf_count);
        let mut current_level: usize = height;

        let mut prev_level_len: usize = 0;
        let mut current_level_len: usize = self.leaf_count;

        // This cache exists to avoid taking multiple mutable borrows on self.nodes
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
        }
        self.nodes.iter().last()
    }
    /// Returns the opening for the tree.
    /// Opening - A list of all partner nodes with which when hashed together computes to the root.
    pub fn get_opening(self: &Self, leaf_index: usize) -> Result<Vec<Hash>, MerkleTreeError> {
        if leaf_index >= self.leaf_count {
            return Err(MerkleTreeError::LeafIndexOutOfBounds(format!(
                "Tree has {} leaves but index given was {}",
                self.leaf_count, leaf_index
            )));
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
            if let Some(lnode) = left_node {
                path.push(lnode);
            }

            if let Some(rnode) = right_node {
                path.push(rnode);
            }

            if current_index % 2 == 0 {
                if current_index + 1 < current_level_len {
                    right_node = Some(current_level_nodes[current_index + 1]);
                } else {
                    right_node = Some(current_level_nodes[current_index]);
                }
                left_node = None;
            } else {
                left_node = Some(current_level_nodes[current_index - 1]);
                right_node = None;
            }
            current_index /= 2;
            prev_level_len += current_level_len;
            current_level_len = Self::calculate_next_level_len(current_level_len);
            current_level -= 1;

            current_level_nodes = &self.nodes[prev_level_len..(prev_level_len + current_level_len)];
        }

        Ok(path)
    }

    /// Returns a bool in a result signifying if the opening is valid and computes to the given root.
    pub fn verify_opening(
        self: &Self,
        opening: Vec<Hash>,
        root: Hash,
        leaf_index: usize,
    ) -> Result<bool, MerkleTreeError> {
        if leaf_index >= self.leaf_count {
            return Err(MerkleTreeError::LeafIndexOutOfBounds(format!(
                "Tree has {} leaves but index given was {}",
                self.leaf_count, leaf_index
            )));
        }

        let leaf = self.nodes[leaf_index];
        let mut computed_root = Hash::default();
        for (i, item) in opening.into_iter().enumerate() {
            if i == 0 {
                // Since the opening doesn't contain the leaf node
                computed_root = hash_node!(item, leaf);
            } else {
                computed_root = hash_node!(item, computed_root)
            }
        }
        Ok(computed_root == root)
    }
}

#[derive(Error, Debug)]
pub enum MerkleTreeError {
    #[error("leaf index out of bounds")]
    LeafIndexOutOfBounds(String),
    #[error("Root not computed")]
    RootNotComputed(String),
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    use solana_program::hash::Hash;

    pub const SAMPLE: &[&[u8]] = &[
        b"lorem",
        b"ipsum",
        b"dolor",
        b"sit",
        b"amet",
        b"consectetur",
        b"adipiscing",
        b"elit",
        b"Integer",
        b"iaculis",
    ];

    pub const EXPECTED: &str = "GWfs1rXnMA3AywiAEYq54Ms5MdB2esdCnVHK2j6SdMdY";

    #[test]
    fn test_calculate_valid_capacity() {
        assert_eq!(MerkleTree::calculate_max_capacity(0), 0);
        assert_eq!(MerkleTree::calculate_max_capacity(1), 1);
        assert_eq!(MerkleTree::calculate_max_capacity(2), 3);
        assert_eq!(MerkleTree::calculate_max_capacity(3), 6);
        assert_eq!(MerkleTree::calculate_max_capacity(4), 7);
        assert_eq!(MerkleTree::calculate_max_capacity(6), 12);
        assert_eq!(MerkleTree::calculate_max_capacity(11), 23);
        assert_eq!(MerkleTree::calculate_max_capacity(16), 31);
        assert_eq!(MerkleTree::calculate_max_capacity(1024), 2047);
    }
    #[test]
    fn test_calculate_valid_height() {
        assert_eq!(MerkleTree::calculate_height(0), 0);
        assert_eq!(MerkleTree::calculate_height(1), 0);
        assert_eq!(MerkleTree::calculate_height(5), 3);
        assert_eq!(MerkleTree::calculate_height(1024), 10);
    }
    #[test]
    fn test_valid_merkle_root() {
        let mut merkle_tree = MerkleTree::new(SAMPLE.len());

        for leaf in SAMPLE {
            let _ = merkle_tree.insert(leaf);
        }

        let root = merkle_tree.get_root();
        matches!(root, Some(_));
        assert_eq!(root.unwrap().to_string(), EXPECTED.to_string());
    }
    #[test]
    fn test_valid_opening() {
        let mut merkle_tree = MerkleTree::new(SAMPLE.len());

        for leaf in SAMPLE {
            let _ = merkle_tree.insert(leaf);
        }
        let _ = merkle_tree.get_root();

        let opening = merkle_tree.get_opening(9).unwrap();
        assert_eq!(opening.len(), 4);
        let is_valid = merkle_tree.verify_opening(opening, Hash::from_str(EXPECTED).unwrap(), 9);
        assert!(is_valid.is_ok());
        assert!(is_valid.unwrap())
    }
    #[test]
    fn test_invalid_index_opening() {
        let mut merkle_tree = MerkleTree::new(SAMPLE.len());

        for leaf in SAMPLE {
            let _ = merkle_tree.insert(leaf);
        }
        let _ = merkle_tree.get_root();

        let opening = merkle_tree.get_opening(10);
        matches!(opening, Err(_));
    }

    #[test]
    fn test_invalid_verify_opening() {
        let mut merkle_tree = MerkleTree::new(SAMPLE.len());

        for leaf in SAMPLE {
            let _ = merkle_tree.insert(leaf);
        }
        let _ = merkle_tree.get_root();

        let opening = merkle_tree.get_opening(9).unwrap();
        let is_valid = merkle_tree.verify_opening(opening, Hash::new_unique(), 9);
        assert!(is_valid.is_ok());
        assert!(!is_valid.unwrap())
    }
}

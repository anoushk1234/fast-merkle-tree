use {
    elusiv_merkle_tree::MerkleTree as ElusivMerkleTree, glassbench::*, lipsum::lipsum,
    solana_merkle_tree::MerkleTree as SolanaMerkleTree, solana_sdk::signature::Signature,
};

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
fn benchmark_merkle_tree(b: &mut Bench) {
    let mut leaves = vec![];
    let leaf_count: usize = 1024;
    for i in 0..leaf_count {
        leaves.push(Signature::new_unique().to_string().as_bytes().to_owned());
    }
    // println!("{}", lipsum(25));

    b.task(
        format!("elusiv-merkle-tree | {} leaves", leaf_count),
        |task| {
            task.iter(|| {
                let mut merkle_tree = ElusivMerkleTree::new(leaf_count);
                for leaf in leaves.clone() {
                    merkle_tree.insert(leaf);
                }
                let root = merkle_tree.get_root();
            });
        },
    );

    b.task(
        format!("solana-merkle-tree | {} leaves", leaf_count),
        |task| {
            task.iter(|| {
                let solana_merkle = SolanaMerkleTree::new(leaves.as_slice());
                let root = solana_merkle.get_root();
            });
        },
    );
}

glassbench!("My Merkle Tree v/s Solana's", benchmark_merkle_tree,);

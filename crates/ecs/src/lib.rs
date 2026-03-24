use tiger_pkg::TagHash;

pub mod interactibles;
pub mod permutations;
pub mod transform;
pub mod world;

pub struct UnimplementedTigerComponent {
    pub class_id: u32,
    pub hash: TagHash,
    pub name: Option<String>,
}

pub struct UnimplementedTigerComponents(pub Vec<UnimplementedTigerComponent>);

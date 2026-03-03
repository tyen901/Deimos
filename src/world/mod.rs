use tiger_pkg::TagHash;

pub mod map;
pub mod pattern;
// pub mod sequencer;
// pub mod shadowmap;

#[allow(unused)]
pub struct UnimplementedTigerComponent {
    pub class_id: u32,
    pub hash: TagHash,
    pub name: Option<String>,
}

pub struct UnimplementedTigerComponents(pub Vec<UnimplementedTigerComponent>);

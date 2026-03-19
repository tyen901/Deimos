use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

#[tiger_type(id = 0x8080B128, size = 0x34)]
pub struct SWwiseEvent {
    pub file_size: u64,
    // pub unk_action_id: u32,
    // pub unkc: u32,
    // pub unk10: TagHash,
    // pub wwise_bank: TagHash,
    #[tiger(offset = 0x20)]
    pub wwise_streams: Vec<TagHash>,
    // pub unk28: TagHash,
}

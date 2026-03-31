use tiger_parse::{tiger_type, tiger_variant_enum, VariantPointer};

#[tiger_type(id = 0x8080AA3B, size = 0x28)]
pub struct S8080AA3B {
    #[tiger(offset = 0x20)]
    pub pointer: VariantPointer<DialogueComponent>,
}

tiger_variant_enum! {
    #[derive(Debug, Clone)]
    [Unknown(true)]
    enum DialogueComponent {
        S8080AA3F
    }
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080AA3F, size = 0xA0)]
pub struct S8080AA3F {
    pub unk: [u32; 40],
}

#[derive(Debug)]
pub enum Metadata {
    Module {
        qualified_name: String,
        name: String,
        docs: String,
    },
    Static {},
    Const {},
    Enum {},
    Struct {},
    Union {},
    Trait {},
    Function {},
    Macro {},
    Tuple {},
    Method {},
    Type {},
    Local {},
    Field {},
}

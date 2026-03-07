pub(crate) mod arg_type;
pub(crate) mod argument;
pub(crate) mod macros;
// pub mod maskfile;
mod parser;

pub use clap;
pub use mask_types;
pub use parser::parse;

use clap::{Arg, value_parser as vp};

macro_rules! match_set_value_parser {
    {
        $builder:ident :
        match $val:ident {
            $(
            $($match_item:literal)|+ =>  $value_parser:expr,
            )+
        }
    } => {
        match $val {
            $(
            $($match_item)|+ => $builder.value_parser($value_parser) ,
            )+
            _ => $builder
        }
    }
}

pub(crate) fn parse(arg: Arg, ty: String) -> Arg {
    let is_list;
    let ty = if ty.starts_with('[') && ty.ends_with(']') {
        is_list = true;
        let chars = ty.chars().collect::<Vec<_>>();
        chars.into_iter().skip(1).rev().skip(1).rev().collect()
    } else {
        is_list = false;
        ty
    };
    let ty = ty.as_str();
    let arg = match_set_value_parser! {
        arg:
        match ty {
            //   - [Native types][ValueParser]: `bool`, `String`, `OsString`, `PathBuf`
            "" | "bool" | "!bool" => vp!(bool),
            "String" => vp!(String),
            "OsString" => vp!(std::ffi::OsString),
            "PathBuf" => vp!(std::path::PathBuf),
            //   - [Ranged numeric types][RangedI64ValueParser]: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`
            "u8" => vp!(u8),
            "u16" => vp!(u16),
            "u32" => vp!(u32),
            "u64" => vp!(u64),
            "i8" => vp!(i8),
            "i16" => vp!(i16),
            "i32" => vp!(i32),
            "i64" => vp!(i64),
            // - [`FromStr` types][std::str::FromStr], including usize, isize
            "usize" => vp!(usize),
            "isize" | "Number" => vp!(isize),
            "f32" => vp!(f32),
            "f64" => vp!(f64),
            // - [`ValueEnum` types][crate::ValueEnum]
            // - [`From<OsString>` types][std::convert::From] and [`From<&OsStr>` types][std::convert::From]
            // - [`From<String>` types][std::convert::From] and [`From<&str>` types][std::convert::From]
            // - [`ValueParserFactory` types][ValueParserFactory], including
        }
    };
    if matches!(ty, "" | "bool") {
        arg.action(clap::ArgAction::SetTrue)
    } else if ty == "bool" {
        arg.action(clap::ArgAction::SetFalse)
    } else if is_list {
        arg.action(clap::ArgAction::Append)
    } else {
        arg.action(clap::ArgAction::Set)
    }
}

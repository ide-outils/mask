use std::mem;

use clap::Arg;

macro_rules! fill_flag {
    {
        $builder:ident
        if $condition:ident { $method_first:ident } else { $method_after:ident }
        = $content:expr
    } => {
        $builder = if $condition.is_none() {
            *$condition = Some($content.clone());
            $builder.$method_first($content)
        } else {
            $builder.$method_after($content)
        };
    }
}

#[derive(Default)]
struct Parser {
    first_long: Option<String>,
    first_short: Option<char>,
    index: usize,
}
impl Parser {
    fn new(index: usize) -> Self {
        Self {
            index,
            ..Default::default()
        }
    }

    fn parse_name(&mut self, arg: Arg, configured_name: String) -> Arg {
        // extract config, flag_config and name (order of seeing)
        let (config, name) = extract_config(configured_name);
        let (config, flag_config) = extract_flag_config(config);
        // Extract and set value_name
        let (mut arg, name) = parse_value_name(arg, name);
        if flag_config.is_empty() {
            // if positional, we must set the index ans the id.
            arg = arg.id(name.clone()).index(self.index);
            debug_assert_eq!(arg.get_id(), &name);
            debug_assert_eq!(arg.get_index(), Some(self.index));
        } else {
            arg = self.parse_flag(arg, flag_config, name.clone());
        }
        for char in config {
            // Put some mnemonics on right eye comment.
            //
            // Any changement, must related in function `extract_config`'s chars's match
            match char {
                '!' => arg = arg.required(true),
                '?' => arg = arg.required(false),
                '$' => arg = arg.last(true),             // $ like last line char in regex
                '*' => arg = arg.trailing_var_arg(true), // like *var in python
                '^' => arg = arg.exclusive(true),        // like first char in regex
                '%' => arg = arg.global(true),           // :% like whole buffer in vim
                '~' => arg = arg.ignore_case(true),      // ~ like toggle case in vim
                '_' => arg = arg.allow_negative_numbers(true), // because - not allowed, let's use _
                '=' => arg = arg.require_equals(true),   //
                '#' => arg = arg.raw(true),              // # like in rust raw r#"strings"#
                '@' => arg = arg.env(name.clone()),      // Sent @ Env
                '.' => arg = arg.hide(true),             // Looks like the smallest visible char.
                // Many other hide methods exists...
                _ => (),
            }
        }
        arg
    }
    fn parse_flag(&mut self, mut arg: Arg, config: String, name: String) -> Arg {
        if name != "" {
            return arg;
        }
        let first_long = &mut self.first_long;
        let first_short = &mut self.first_short;
        match config.as_str() {
            "--?" => {
                if name != "" {
                    fill_flag! { arg if first_long { long } else { alias } = name }
                }
            }
            "--" => {
                if name != "" {
                    fill_flag! { arg if first_long { long } else { visible_alias } = name }
                }
            }
            "-?" => {
                if let Some(char) = name.chars().next() {
                    fill_flag! { arg if first_short { short } else { short_alias } = char }
                }
            }
            "-" => {
                if let Some(char) = name.chars().next() {
                    fill_flag! { arg if first_short { short } else { visible_short_alias } = char }
                }
            }
            _ => (),
        }
        arg
    }
}
pub(crate) fn parse(mut arg: Arg, chars: &mut impl Iterator<Item = char>, index: usize) -> (Arg, Option<char>) {
    // RECAP: - `: `  for `Arg::value_parser`
    //        - `= `  for `Arg::default_value`
    //        - `=? ` for `Arg::default_missing_value`
    //        - `=>`  for `Arg::value_hint`
    let mut parser = Parser::new(index);
    // let mut chars = names.chars();
    type Name = String;
    type Names = Vec<Name>;
    let mut names = Names::new();
    let current_name = &mut Name::new();
    let mut prev = None;
    let mut in_names = true;
    let push_name = |names: &mut Names, current_name: &mut Name| {
        let name = mem::take(current_name);
        let name = name.trim().to_string();
        if name != "" {
            names.push(name)
        }
    };
    loop {
        let Some(char) = chars.next() else {
            prev = None;
            break;
        };
        match (prev, char) {
            (Some(':'), ' ') => {
                let word = take_until_space(chars);
                arg = crate::arg_type::parse(arg, word);
                in_names = false;
            }
            (Some('='), ' ') => {
                let word = take_until_space(chars);
                arg = parse_default_value(arg, word);
                in_names = false;
            }
            (Some('='), '>') => {
                let word = take_until_space(chars);
                arg = parse_value_hint(arg, word);
                in_names = false;
            }
            (Some('='), '?') => match chars.next() {
                Some(' ') => {
                    let word = take_until_space(chars);
                    arg = parse_default_missing_value(arg, word);
                    in_names = false;
                }
                // Then it is a name
                Some(c) => {
                    if in_names {
                        current_name.push(c);
                    } else {
                        prev = Some(c);
                        break;
                    }
                }
                None => {
                    prev = None;
                    break;
                }
            },
            _ => (),
        }

        match char {
            // Next argument's separator
            // If you need to use it here, then you must either
            //  - count `"` % 2 == 1
            //  - count `'` % 2 == 1
            //  - or check for backslash in prev (and in case remove it from names)
            ',' => {
                prev = Some(char);
                break;
            }

            // Next group's separator
            // If you need to use it here, then you must inc/dec open/close
            ')' | ']' | '}' => {
                prev = Some(char);
                break;
            }

            // Next name's separator
            // We might use space in <>, but it requires to count it..
            ' ' if in_names => push_name(&mut names, current_name),

            // Fill current name (config + name)
            c if in_names => current_name.push(c),

            // All names are done, see double char match above.
            _ => (),
        }
        prev = Some(char);
    }
    push_name(&mut names, current_name);
    for name in names {
        arg = parser.parse_name(arg, name);
    }
    if arg.get_index().is_none() {
        let id = parser
            .first_long
            .or(parser.first_short.map(|c| c.to_string()))
            .unwrap_or("ErrorNoFlagNoPositional".to_string());
        // FIXME: Return an error instead
        // Then the caller of this function will need to remove the arg
        // Another smooth solution, would be to add a field `errors` in `MaskData`
        arg = arg.id(id);
    }
    (arg, prev)
}

fn take_until_space(chars: &mut impl Iterator<Item = char>) -> String {
    let mut word = String::new();
    while let Some(char) = chars.next() {
        if char.is_whitespace() {
            break;
        } else {
            word.push(char);
        }
    }
    word.trim().to_string()
}

fn parse_default_value(arg: Arg, value: String) -> Arg {
    arg.default_value(value)
}
fn parse_default_missing_value(arg: Arg, value: String) -> Arg {
    arg.default_missing_value(value)
}
fn parse_value_hint(arg: Arg, hint: String) -> Arg {
    if let Ok(hint) = <clap::ValueHint as std::str::FromStr>::from_str(&hint) {
        arg.value_hint(hint)
    } else {
        arg
    }
}

fn parse_value_name(mut arg: Arg, name: String) -> (Arg, String) {
    let mut names = name.split("<");
    let name_parsed = names.next().unwrap().trim().to_string();
    // Example :
    // --manger <Légume> <Gibier> <Féculent>
    // all spaces will be trimmed
    while let Some(value_name) = names.next()
        && value_name.trim().ends_with(">")
    {
        arg = arg.value_name(value_name.trim().to_string());
    }
    (arg, name_parsed)
}

fn extract_config(configured_name: String) -> (Vec<char>, String) {
    let mut config = Vec::new();
    let mut chars = configured_name.chars();
    let mut name = String::new();
    while let Some(char) = chars.next() {
        if matches!(
            char,
            // Specific to flags
            '-' |
            // Config chars.
            '!' | '?' | '$' | '*' | '^' | '%' | '~' | '_' | '=' | '#' | '@' | '.'
        ) {
            config.push(char);
        } else {
            name.push(char);
            break;
        }
    }
    name.push_str(&chars.collect::<String>());
    (config, name)
}

fn extract_flag_config(config: Vec<char>) -> (Vec<char>, String) {
    let mut chars_config = config.into_iter();
    let mut flag_config = String::new();
    let mut config = Vec::new();
    while let Some(char) = chars_config.next() {
        if char == '-' {
            flag_config.push(char);
            break;
        } else {
            config.push(char);
        }
    }
    flag_config.push_str(&chars_config.collect::<String>());
    (config, flag_config)
}

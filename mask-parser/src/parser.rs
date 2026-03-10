use std::{cell::RefCell, mem};

use clap::{Arg, ArgGroup, Command};
use mask_types::{Mask, Script};
use pulldown_cmark::{
    Event::{Code, End, InlineHtml, Start, Text},
    Options, Parser, Tag,
};

use crate::macros::{mask_read, mask_write, set};

pub fn parse(maskfile_contents: String) -> Command {
    let parser = create_markdown_parser(&maskfile_contents);
    let mut commands: Vec<Command> = vec![];
    let current_command = &mut Command::new("");
    *current_command = mem::take(current_command).add(Mask::new(0));
    let current_option_flag = &mut Arg::new("");
    let text = RefCell::new(String::new());
    let mut list_level = 0;
    let mut first = true;

    for event in parser {
        match event {
            Start(tag) => {
                match tag {
                    Tag::Header(heading_level) => {
                        let mut command = std::mem::take(current_command);
                        if !first {
                            if commands.len() == 0 {
                                let bin_name = command.get_name().to_string();
                                command = command.bin_name(bin_name).name("")
                            }
                            commands.push(command);
                        } else {
                            // Do not add the not parsed first command.
                            first = false;
                        }
                        *current_command = mem::take(current_command).add(Mask::new(heading_level));
                    }
                    Tag::CodeBlock(_lang_code) => {
                        // We don't care it is open, we want it to be closed.
                    }
                    Tag::List(_) => {
                        // We're in an options list if the current text above it is "OPTIONS"
                        if *text.borrow() == "OPTIONS" || list_level > 0 {
                            list_level += 1;
                        }
                    }
                    _ => (),
                };

                // Reset all state
                text.borrow_mut().clear();
            }
            End(tag) => match tag {
                Tag::Header(_level) => {
                    let (names, args, groups) = parse_command_name_required_and_optional_args(text.take());
                    let (name, aliases) = parse_command_name_and_aliases(names);
                    *current_command = mem::take(current_command)
                        .name(name)
                        .args(args)
                        .aliases(aliases)
                        .groups(groups);
                }
                Tag::BlockQuote => {
                    set! { current_command.about = text.take() };
                }
                Tag::CodeBlock(lang_code) => {
                    let script = Script {
                        lang_code: lang_code.to_string(),
                        content: text.take(),
                    };
                    mask_write!(current_command).scripts.push(script);
                }
                Tag::List(_) => {
                    // Don't go lower than zero (for cases where it's a non-OPTIONS list)
                    list_level = std::cmp::max(list_level - 1, 0);

                    // Must be finished parsing the current option
                    if list_level == 1 {
                        // Add the current one to the list and start a new one
                        let arg = mem::take(current_option_flag);
                        set! { current_command.arg = arg };
                    }
                }
                _ => (),
            },
            Text(body) => {
                text.borrow_mut().push_str(&body);

                // Options level 1 is the flag name
                if list_level == 1 {
                    set! { current_option_flag.id = text.take() };
                }
                // Options level 2 is the flag config
                else if list_level == 2 {
                    let content = text.take();
                    let mut config_split = content.splitn(2, ":");
                    let param = config_split.next().unwrap_or("").trim();
                    let val = config_split.next().unwrap_or("").trim().to_string();
                    match param {
                        //   param        val
                        //   -----  -----------------
                        // * desc:  Décrocher la lune
                        "desc" => set! {current_option_flag.help = val.to_string() },
                        //   param   val
                        //   -----  -----
                        // * type:  usize
                        "type" => {
                            *current_option_flag = crate::arg_type::parse(std::mem::take(current_option_flag), val)
                        }
                        // Parse out the short and long flag names
                        //
                        //   param                   val
                        //   -----  ------------------------------------------------
                        // * flags: --long -s -a -?u --?unvisible-alias -alias --etc
                        "flags" => {
                            let val = val.replace(' ', "|"); // argument::parse expect '|' as a separator
                            let (new_arg, _) =
                                crate::argument::parse(std::mem::take(current_option_flag), &mut val.chars(), 0);
                            *current_option_flag = new_arg;
                        }
                        //    param           val
                        //   -------   -----------------
                        // * choices:  Un | Deux | Trois
                        "choices" => {
                            todo!(
                                "We should apparently create a validator by setting `value_parser` problably a function."
                            )
                        }
                        //    param    val
                        //   --------
                        // * required
                        "required" => {
                            set! { current_option_flag.required = true };
                        }
                        _ => (),
                    };
                }
            }
            InlineHtml(html) => {
                text.borrow_mut().push_str(&html);
            }
            Code(inline_code) => {
                text.borrow_mut()
                    .push_str(&format!("`{}`", inline_code));
            }
            _ => (),
        };
    }
    commands.push(mem::take(current_command));
    let mut root = commands.remove(0);
    let (subcommands, _) = treeify_commands(&mut root, &mut commands.into_iter());
    root.subcommands(subcommands)
}

fn create_markdown_parser<'a>(maskfile_contents: &'a String) -> Parser<'a> {
    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&maskfile_contents, options);
    parser
}

fn treeify_commands<'static_or_freed>(
    command: &mut Command,
    left_cmds: &mut impl Iterator<Item = Command>,
) -> (Vec<Command>, Option<Command>) {
    let command_level = mask_read!(command).level;
    let command_name = command.get_name();
    let mut retenue = None;
    let mut subcommands = Vec::new();

    while let Some(mut sub) = retenue.take().or_else(|| left_cmds.next()) {
        // Firstly ensure it is a sub Command of `command`
        // Otherwise return with it as a reminder/retenue
        let sub_level = mask_read!(sub).level;
        if sub_level <= command_level {
            // Found a sibling or an ancestor, so the current command has found all children.
            if command_level == 1 {
                // If another root exists, simply skip it, we might find more commands after.
                continue;
            }
            retenue = Some(sub);
            break;
        }
        // then it is a sub command
        let sub_name = sub.get_name().to_string();
        if sub_name.starts_with(&command_name) {
            // Remove parent command name prefixes from subcommand
            let stripped_name = sub_name.strip_prefix(&command_name).unwrap().trim();
            sub = sub.name(stripped_name.to_string());
        }
        let (sub_subcommands, ret) = treeify_commands(&mut sub, left_cmds);
        sub = sub.subcommands(sub_subcommands); // performs Vec::extends
        retenue = ret;
        subcommands.push(sub);
    }
    subcommands.retain(|c| {
        let m = mask_read!(c);
        !m.scripts.is_empty() || c.get_subcommands().count() > 0 || m.level == 1
    });
    (subcommands, retenue)
}

fn parse_command_name_and_aliases(text: String) -> (String, Vec<String>) {
    let mut aliases_it = text.split('|').map(|name| name.trim().to_string());
    (
        // Split generates at least one item
        Option::unwrap(aliases_it.next()),
        aliases_it.collect(),
    )
}

fn parse_command_name_required_and_optional_args(text: String) -> (String, Vec<Arg>, Vec<ArgGroup>) {
    // Checks if any args are present and if not, return early
    let split_idx = match text.find(|c| c == '(' || c == '[') {
        Some(idx) => idx,
        None => return (text.trim().to_string(), vec![], vec![]),
    };

    let (name, args_str) = text.split_at(split_idx);
    let name = name.trim().to_string();

    let mut arguments = vec![];
    let mut groups = vec![];

    let mut chars = args_str.chars();
    let group_name = &mut String::new();
    while let Some(char) = chars.next() {
        match char {
            '(' | '[' => {
                let required = char == '(';
                let mut args = Vec::new();
                let gname = mem::take(group_name);
                'group: loop {
                    let arg = Arg::new("").required(required).group(gname.clone());
                    // println!("argument::parse : ");
                    let index = args.len();
                    let (new_arg, prev_char) = crate::argument::parse(arg, &mut chars, index);
                    args.push(new_arg);
                    let Some(prev_char) = prev_char else { break 'group };
                    if matches!(prev_char, ')' | ']') {
                        if group_name != "" {
                            let ids = args.iter().map(|a| a.get_id());
                            let group = ArgGroup::new(gname).args(ids);
                            groups.push(group);
                        }
                        arguments.extend(args);
                        group_name.clear();
                        break;
                    }
                    debug_assert_eq!(prev_char, ',');
                }
            }
            c => {
                if c == ' ' {
                    group_name.clear();
                } else {
                    group_name.push(c);
                }
            }
        }
    }
    (name, arguments, groups)
}

#[cfg(test)]
const TEST_MASKFILE: &str = r#"
# Document Title

This is an example maskfile for the tests below.

## serve (port)

> Serve the app on the `port`

~~~bash
echo "Serving on port $port"
~~~

## node (name)

> An example node script

Valid lang codes: js, javascript

```js
const { name } = process.env;
console.log(`Hello, ${name}!`);
```

## parent
### parent subcommand | alias
> This is a subcommand

~~~bash
echo hey
~~~

## no_script

This command has no source/script.

## multi (required) [optional]

> Example with optional args

~~~bash
if ! [ -z "$optional" ]; then
 echo "This is optional - $optional"
fi

echo "This is required - $required"
~~~

# This is an invalid H1, so it will be ignored

## invalid_cmd

Everything below an invalid H1 will also be ignored.

~~~bash
echo hey
~~~
"#;

#[cfg(test)]
mod tests_legacy {
    use serde::ser::SerializeStruct as _;
    use serde_json::json;

    use super::*;
    use crate::macros::tests::file_assert_eq;

    struct LegacyScript<'ser>(&'ser Script);
    impl<'ser> serde::Serialize for LegacyScript<'ser> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut script_ser = serializer.serialize_struct("Script", 2)?;
            script_ser.serialize_field("executor", &self.0.lang_code)?;
            script_ser.serialize_field("source", &self.0.content)?;
            script_ser.end()
        }
    }

    #[derive(serde::Serialize)]
    struct LegacyArg {
        name: String,
    }
    impl LegacyArg {
        fn new(name: String) -> Self {
            Self { name }
        }
    }
    struct LegacyMaskFile<'ser>(&'ser Command);
    impl<'ser> LegacyMaskFile<'ser> {
        pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            serde_json::to_value(&self)
        }
    }
    impl<'ser> serde::Serialize for LegacyMaskFile<'ser> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let command = &self.0;
            let mut ms = serializer.serialize_struct("MaskFile", 1)?;
            ms.serialize_field("title", &command.get_bin_name())?;
            let about = &command.get_about();
            let desc = about.map_or("".to_string(), |a| a.to_string());
            ms.serialize_field("description", &desc)?;
            let subcommands = command.get_subcommands();
            ms.serialize_field(
                "commands",
                &subcommands
                    .map(|cmd| LegacySerializer(cmd))
                    .collect::<Vec<_>>(),
            )?;
            ms.end()
        }
    }

    struct LegacySerializer<'ser>(&'ser Command);
    impl<'ser> serde::Serialize for LegacySerializer<'ser> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let command = &self.0;
            let mask = mask_read!(command);
            let mut ms = serializer.serialize_struct("Command", 1)?;
            ms.serialize_field("level", &mask.level)?;
            ms.serialize_field("name", &command.get_name())?;
            let about = &command.get_about();
            let desc = about.map_or("".to_string(), |a| a.to_string());
            ms.serialize_field("description", &desc)?;
            ms.serialize_field("script", &mask.scripts.get(0).map(|s| LegacyScript(s)))?;
            let subcommands = command.get_subcommands();
            ms.serialize_field("subcommands", &subcommands.map(|cmd| Self(cmd)).collect::<Vec<_>>())?;
            let required_args = command
                .get_arguments()
                .filter_map(|a| {
                    if a.is_positional() && a.is_required_set() {
                        let name = a.get_id().to_string();
                        Some(LegacyArg::new(name))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            ms.serialize_field("required_args", &required_args)?;
            let optional_args = command
                .get_arguments()
                .filter_map(|a| {
                    if a.is_positional() && !a.is_required_set() {
                        let name = a.get_id().to_string();
                        Some(LegacyArg::new(name))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            ms.serialize_field("optional_args", &optional_args)?;
            let named_flags = command
                .get_arguments()
                .filter_map(|a| {
                    if !a.is_positional() {
                        Some(
                            a.get_long_and_visible_aliases()
                                .unwrap()
                                .into_iter()
                                .map(|s| s.to_string())
                                .chain(
                                    a.get_short_and_visible_aliases()
                                        .unwrap()
                                        .into_iter()
                                        .map(|s| s.to_string()),
                                ),
                        )
                    } else {
                        None
                    }
                })
                .flatten()
                .collect::<Vec<_>>();
            ms.serialize_field("named_flags", &named_flags)?;
            ms.end()
        }
    }

    #[test]
    fn legacy_parses_the_maskfile_structure() {
        let root = parse(TEST_MASKFILE.to_string());

        let verbose_flag = json!({
            "name": "verbose",
            "description": "Sets the level of verbosity",
            "short": "v",
            "long": "verbose",
            "multiple": false,
            "takes_value": false,
            "required": false,
            "validate_as_number": false,
            "choices": [],
        });

        file_assert_eq!(
            json!({
                "title": "Document Title",
                "description": "",
                "commands": [
                    {
                        "level": 2,
                        "name": "serve",
                        "description": "Serve the app on the `port`",
                        "script": {
                            "executor": "bash",
                            "source": "echo \"Serving on port $port\"\n",
                        },
                        "subcommands": [],
                        "required_args": [
                            {
                                "name": "port"
                            }
                        ],
                        "optional_args": [],
                        "named_flags": [],
                        // "named_flags": [verbose_flag],
                    },
                    {
                        "level": 2,
                        "name": "node",
                        "description": "An example node script",
                        "script": {
                            "executor": "js",
                            "source": "const { name } = process.env;\nconsole.log(`Hello, ${name}!`);\n",
                        },
                        "subcommands": [],
                        "required_args": [
                            {
                                "name": "name"
                            }
                        ],
                        "optional_args": [],
                        "named_flags": [],
                        // "named_flags": [verbose_flag],
                    },
                    {
                        "level": 2,
                        "name": "parent",
                        "description": "",
                        "script": null,
                        "subcommands": [
                            {
                                "level": 3,
                                "name": "subcommand",
                                "description": "This is a subcommand",
                                "script": {
                                    "executor": "bash",
                                    "source": "echo hey\n",
                                },
                                "subcommands": [],
                                "optional_args": [],
                                "required_args": [],
                                "named_flags": [],
                                // "named_flags": [verbose_flag],
                            }
                        ],
                        "required_args": [],
                        "optional_args": [],
                        "named_flags": [],
                    },
                    {
                        "level": 2,
                        "name": "multi",
                        "description": "Example with optional args",
                        "script": {
                            "executor": "bash",
                            "source": "if ! [ -z \"$optional\" ]; then\n echo \"This is optional - $optional\"\nfi\n\necho \"This is required - $required\"\n",
                        },
                        "subcommands": [],
                        "required_args": [{ "name": "required" }],
                        "optional_args": [{ "name": "optional" }],
                        "named_flags": [],
                        // "named_flags": [verbose_flag],
                    }
                ]
            }),
            LegacyMaskFile(&root)
                .to_json()
                .expect("should have serialized to json"),
            "Not the expected JSon output."
        );
    }
}

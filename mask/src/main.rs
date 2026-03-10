mod executor;
mod loader;
use std::{ffi::OsStr, fs, io};

use mask_parser::clap::{Arg, ArgMatches, Command};

struct Selection<'root, 'matches> {
    commands: Vec<&'root Command>,
    arguments: Vec<ArgItems<'root, 'matches>>,
}
struct ArgItems<'root, 'matches> {
    arg: &'root Arg,
    items: Option<Vec<Vec<&'matches OsStr>>>,
}

fn main() -> io::Result<()> {
    let config = loader::load()?;
    let root = mask_parser::parse(fs::read_to_string(&config.path)?);
    let root = config.improve_helper(root);
    let mut root = loader::root_setup(root).subcommand_required(true);
    config.run_skipped_help(&mut root)?;
    let matches = root.get_matches_mut();
    let selection = apply_matches(&matches, &root);
    if let Some(code) = selection.execute_command(config)?.code() {
        std::process::exit(code)
    } else {
        eprintln!("Has been stopped by a signal.");
        Ok(())
    }
}

fn apply_matches<'root: 'matches, 'matches: 'root>(
    mut matches: &'matches ArgMatches,
    mut parent: &'root Command,
) -> Selection<'root, 'matches> {
    let mut sub_commands = parent.get_subcommands().into_iter();
    let mut commands = Vec::new();
    let mut arguments = Vec::new();
    arguments.extend(filter_args(&matches, parent));
    'matches: while let Some((name, sub_matches)) = matches.subcommand() {
        '_commands: while let Some(sub_cmd) = sub_commands.next() {
            if sub_cmd.get_name() == name {
                parent = sub_cmd;
                commands.push(sub_cmd);
                matches = &sub_matches;
                arguments.extend(filter_args(&matches, parent));
                continue 'matches;
            }
        }
    }
    Selection { commands, arguments }
}

fn filter_args<'root>(matches: &'root ArgMatches, cmd: &'root Command) -> impl Iterator<Item = ArgItems<'root, 'root>> {
    cmd.get_arguments()
        .zip(matches.ids())
        .filter(|(arg, id)| arg.get_id() == *id)
        .map(|(arg, id)| ArgItems {
            arg,
            items: matches
                .get_raw_occurrences(id.as_str())
                .map(|some| some.map(Iterator::collect).collect()),
        })
}

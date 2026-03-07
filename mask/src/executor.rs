use std::{
    ffi::OsStr,
    io::{Error, ErrorKind, Result},
    path::Path,
    process::{Command as Child, ExitStatus},
};

use mask_parser::{clap::crate_name, mask_read};
use mask_types::Script;

use crate::{ArgItems, Selection, loader::Config};

impl<'root, 'matches> Selection<'root, 'matches> {
    pub fn execute_command(self, config: Config) -> Result<ExitStatus> {
        let arguments = self.arguments;
        let mut exit_code = ExitStatus::default();
        let directory = config.path.parent().unwrap();
        for cmd in self.commands {
            for script in &mask_read!(cmd).scripts {
                // CLEAN: isn't this empty checks should be filter in parser ?
                if script.content.is_empty() || script.lang_code.is_empty() {
                    let msg = "Command is missing script or lang code which determines which lang_code to use.";
                    return Err(Error::new(ErrorKind::Other, msg));
                }
                let mut child = prepare_command(script, &config);
                add_utility_variables(&mut child, &directory);
                add_flag_variables(&mut child, &arguments);
                exit_code = child
                    .spawn()
                    .map_err(|e| {
                        if e.kind() != ErrorKind::NotFound {
                            return e;
                        }
                        Error::new(
                            ErrorKind::NotFound,
                            format!(
                                "program '{}' for lang_code '{}' not in PATH",
                                child.get_program().to_string_lossy(),
                                script.content
                            ),
                        )
                    })?
                    .wait()?;
            }
        }

        Ok(exit_code)
    }
}

fn prepare_command(script: &Script, config: &Config) -> Child {
    let lang_code = script.lang_code.clone();
    let content = script.content.clone();

    match lang_code.as_ref() {
        "js" | "javascript" => {
            let mut child = Child::new(&config.exec.Javascript);
            child.arg("-e").arg(content);
            child
        }
        "py" | "python" => {
            let mut child = Child::new(&config.exec.Python);
            child.arg("-c").arg(content);
            child
        }
        "lua" => {
            let mut child = Child::new(&config.exec.Lua);
            child.arg("-e").arg(content);
            child
        }
        "rb" | "ruby" => {
            let mut child = Child::new(&config.exec.Ruby);
            child.arg("-e").arg(content);
            child
        }
        "php" => {
            let mut child = Child::new(&config.exec.Php);
            child.arg("-r").arg(content);
            child
        }
        "swift" => {
            let mut child = Child::new(&config.exec.Swift);
            child.arg("-e").arg(content);
            child
        }
        // Any other lang_code that supports -c (sh, bash, zsh, fish, dash, etc...)
        _ => {
            let mut child = Child::new(lang_code);
            child.arg("-c").arg(content);
            child
        }
    }
}

// Add some useful environment variables that scripts can use
fn add_utility_variables(child: &mut Child, dir: &Path) {
    // This allows us to call "$MASK command" instead of "mask --maskfile <path> command"
    // inside scripts so that they can be location-agnostic (not care where they are
    // called from). This is useful for global maskfiles especially.
    child.env(
        "MASK",
        format!("{} --maskfile {}", crate_name!(), dir.to_string_lossy()),
    );
    // This allows us to refer to the directory the maskfile lives in which can be handy
    // for loading relative files to it.
    child.env("MASKFILE_DIR", dir);
}

fn add_flag_variables<'root, 'matches>(child: &mut Child, arguments: &Vec<ArgItems<'root, 'matches>>) {
    for ArgItems { arg, items } in arguments {
        let name = arg.get_id();
        match items {
            Some(items) => {
                let sep_item = arg
                    .get_value_terminator()
                    .map(|s| OsStr::new(s))
                    .unwrap_or(OsStr::new(";"));
                let sep = arg.get_value_delimiter().unwrap_or(',').to_string();
                let sep_os = OsStr::new(&sep);
                let value = items
                    .iter()
                    .map(|item| item.join(sep_os))
                    .collect::<Vec<_>>()
                    .join(sep_item);
                child.env(name.as_str(), value);
            }
            None => (),
        }
    }
}

use std::{env, io, path::PathBuf};

use mask_parser::clap::{
    Arg,
    ArgAction::SetTrue,
    ArgGroup, ArgMatches, ColorChoice, Command, ValueHint,
    builder::{OsStr, Resettable},
    crate_name, crate_version,
    parser::ValueSource,
    value_parser,
};

const OPT_NAME: &str = "maskname";
const OPT_PATH: &str = "maskpath";
const OPT_PATH_HIDDEN: &str = "maskfile";
const OPT_HELP: &str = "maskhelp";

const DEFAULT_FILE_NAME: &str = "maskfile.md";
const DEFAULT_FILE_NAME_ENV: &str = "MASK_DEFAULT_FILE_NAME";
const MASK_PATH_ENV: &str = "MASK_PATH";
const MASK_PATH_MISSING_ENV: &str = "MASK_PATH_MISSING";

fn arg_path() -> Arg {
    Arg::new(OPT_PATH)
        .help("The markdown file to build a CLI with.")
        .long(OPT_PATH)
        .alias(OPT_PATH_HIDDEN)
        .value_parser(value_parser!(std::path::PathBuf))
        .default_value(find_file_from_current_to_root(DEFAULT_FILE_NAME))
        .env(MASK_PATH_ENV)
        .default_missing_value(env_var(MASK_PATH_MISSING_ENV))
        .value_hint(ValueHint::FilePath)
}

fn arg_name() -> Arg {
    Arg::new(OPT_NAME)
        .help("The name of the markdown file to look for.")
        .long(OPT_NAME)
        .value_parser(value_parser!(String))
        .default_value(DEFAULT_FILE_NAME)
        .env(DEFAULT_FILE_NAME_ENV)
}

pub fn load() -> io::Result<Config> {
    // Clean up path
    let (path, config_exec, help) = load_path();
    Ok(Config {
        path: path.canonicalize()?,
        exec: config_exec,
        help,
    })
}

pub struct Help {
    long: bool,
    short: bool,
}
pub struct Config {
    pub path: PathBuf,
    pub exec: ExecConfig,
    pub help: Help,
}
impl Config {
    pub fn improve_helper(&self, root: Command) -> Command {
        let path = self.path.to_string_lossy().to_string();
        root.after_help(&path)
            .after_long_help(&path)
            .about(path)
    }
    pub fn run_skipped_help(&self, cmd: &mut Command) -> io::Result<()> {
        let help = &self.help;
        if help.long {
            cmd.print_long_help()?;
        } else if help.short {
            cmd.print_help()?;
        } else {
            return Ok(());
        }
        std::process::exit(0)
    }
}

fn load_path() -> (PathBuf, ExecConfig, Help) {
    let mut root = root_setup(Command::new(""))
        .disable_help_flag(true)
        .arg(Arg::new("help").long("help").action(SetTrue))
        .arg(Arg::new("h").short('h').action(SetTrue))
        .arg(arg_path())
        .arg(arg_name())
        .group(executables_group())
        .args(executables_args())
        .arg(Arg::new("args").trailing_var_arg(true).num_args(..));
    let mut matches = root.get_matches_mut();
    if matches.get_flag("maskhelp") {
        root.print_long_help().unwrap();
        std::process::exit(0);
    }
    let path_is_default = matches
        .value_source(OPT_PATH)
        .map_or(true, |v| v == ValueSource::DefaultValue);
    let help = Help {
        long: matches.get_flag("help"),
        short: matches.get_flag("h"),
    };
    let path = matches.remove_one(OPT_PATH);
    let name = matches
        .get_one::<String>(OPT_NAME)
        .expect("OPT_NAME has a DefaultValue");
    let config_exec = ExecConfig::from_matches(&matches);
    if !path_is_default && let Some(path) = path {
        (path, config_exec, help)
    } else {
        let Some(path) = find_file(name) else {
            root.print_help().unwrap();
            std::process::exit(1);
        };
        (path, config_exec, help)
    }
}

pub fn root_setup(root: Command) -> Command {
    // Command::new(crate_name!())
    root.name(crate_name!())
        .disable_version_flag(true)
        .color(ColorChoice::Always)
        .version(crate_version!())
        .arg(
            Arg::new(OPT_HELP)
                .long(OPT_HELP)
                .action(SetTrue)
                .help("Show the global mask help."),
        )
}

fn env_var(key: &str) -> Resettable<OsStr> {
    env::var(key).ok().map(|s| s.into()).into()
}

// /// Get the target maskfile
// pub fn find_maskfile() -> Resettable<OsStr> {
//     if let Some(path) = find_arg("--maskfile")
//         .or(env::var(MASK_PATH_ENV).ok())
//         .map(|s| OsStr::from(&s))
//     {
//         return Value(path);
//     }
//     let target = find_arg("--maskfile-name")
//         .or(env::var(DEFAULT_FILE_NAME_ENV).ok())
//         .unwrap_or(DEFAULT_FILE_NAME.to_string());
//     find_file_from_current_to_root(&target)
// }

// /// Get the arg following the given target.
// fn find_arg(target: &str) -> Option<String> {
//     let mut args = env::args();
//     while let Some(arg) = args.next() {
//         if arg == target {
//             return args.next();
//         }
//     }
//     None
// }

fn find_file_from_current_to_root(file_name: &str) -> Resettable<OsStr> {
    find_file(file_name)
        .map(|p| p.into_os_string().into())
        .into()
}
fn find_file(file_name: &str) -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(file_name);
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

macro_rules! executables_arg {
    { $($Exec:ident : --$language:ident || $ENV:ident || $DEFAULT:ident = $default:literal)+ } => {
$(
pub const $ENV: &str = stringify!($ENV);
pub const $DEFAULT: &str = $default;
)+
fn executables_args() -> Vec<Arg> {
    vec![
        $(
        Arg::new(stringify!($language))
            .long(stringify!($language))
            .help(format!("Set the executable to run {}'s code block.", stringify!($Lang)))
            .value_name("Exec")
            .value_parser(value_parser!(String))
            .value_hint(ValueHint::CommandName)
            .default_value($DEFAULT)
            .env($ENV),
        )+
    ]
}
fn executables_group() -> ArgGroup {
    ArgGroup::new("executables")
        .args([$(stringify!($language)),+])

}
#[allow(non_snake_case)]
pub struct ExecConfig {
    $(
    pub $Exec: String,
    )+
}
impl ExecConfig {
    fn from_matches(matches: &ArgMatches) -> Self {
        Self {
        $(
            $Exec: matches.get_one::<String>(&stringify!($language)).expect("Get one with default value.").clone(),
        )+
        }

    }
}
}
}
executables_arg! {
    Javascript : --maskjavascript || MASK_EXEC_JAVASCRIPT || DEFAULT_EXEC_JAVASCRIPT = "node"
    Python     : --maskpython     || MASK_EXEC_PYTHON     || DEFAULT_EXEC_PYTHON     = "python"
    Lua        : --masklua        || MASK_EXEC_LUA        || DEFAULT_EXEC_LUA        = "lua"
    Ruby       : --maskruby       || MASK_EXEC_RUBY       || DEFAULT_EXEC_RUBY       = "ruby"
    Php        : --maskphp        || MASK_EXEC_PHP        || DEFAULT_EXEC_PHP        = "php"
    Swift      : --maskswift      || MASK_EXEC_SWIFT      || DEFAULT_EXEC_SWIFT      = "swift"
}

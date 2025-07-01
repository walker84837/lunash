use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use clap::{Parser, Subcommand};

use directories::ProjectDirs;
use mlua::{Lua, LuaOptions, StdLib, UserData, UserDataFields, UserDataMethods};
use regex::Regex;
use reqwest::blocking::Client;

struct FsUtils;
struct StringUtils;
struct RegexWrapper(Regex);
struct ClipboardHandling;

#[allow(dead_code)]
struct HttpModule {
    client: Arc<Mutex<Client>>,
}

impl UserData for FsUtils {
    fn add_fields<'lua, F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("dirname", |_, _| {
            Ok(Path::new(env::current_dir()?.to_str().unwrap())
                .parent()
                .map(|p| p.to_str().unwrap().to_string()))
        });
    }

    fn add_methods<'lua, M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("basename", |_, path: String| {
            Ok(Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string()))
        });

        methods.add_function("readlink", |_, path: String| {
            fs::read_link(&path)
                .map(|p| p.to_str().map(|s| s.to_string()))
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))
        });

        methods.add_function("dirname", |_, path: String| {
            Ok(Path::new(&path)
                .parent()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string()))
        });
    }
}

impl UserData for StringUtils {
    fn add_methods<'lua, M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("split", |_, (s, pat): (String, String)| {
            let parts: Vec<String> = s.split(&pat).map(|p| p.to_string()).collect();
            Ok(parts)
        });

        methods.add_function("trim", |_, s: String| Ok(s.trim().to_string()));
    }
}

impl UserData for RegexWrapper {
    fn add_methods<'lua, M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("is_match", |_, this, text: String| {
            Ok(this.0.is_match(&text))
        });

        methods.add_method("captures", |lua, this, text: String| {
            let table = lua.create_table()?;
            if let Some(caps) = this.0.captures(&text) {
                for (i, cap) in caps.iter().enumerate() {
                    if let Some(cap) = cap {
                        table.set(i + 1, cap.as_str())?;
                    }
                }
            }
            Ok(table)
        });
    }
}

impl UserData for HttpModule {
    fn add_methods<'lua, M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get", |lua, _, url: String| {
            let client = lua
                .app_data_ref::<Arc<Mutex<Client>>>()
                .ok_or_else(|| mlua::Error::RuntimeError("HTTP client not available".into()))?;

            let response = client
                .lock()
                .map_err(|_| mlua::Error::RuntimeError("HTTP client lock poisoned".into()))?
                .get(&url)
                .send()
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

            let text = response
                .text()
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

            lua.create_string(&text)
        });

        methods.add_method("post", |lua, _, (url, body): (String, String)| {
            let client = lua
                .app_data_ref::<Arc<Mutex<Client>>>()
                .ok_or_else(|| mlua::Error::RuntimeError("HTTP client not available".into()))?;

            let response = client
                .lock()
                .map_err(|_| mlua::Error::RuntimeError("HTTP client lock poisoned".into()))?
                .post(&url)
                .body(body)
                .send()
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

            let text = response
                .text()
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

            lua.create_string(&text)
        });
    }
}

impl UserData for ClipboardHandling {
    fn add_methods<'lua, M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("set", |_, _, text: String| {
            let mut clipboard =
                arboard::Clipboard::new().map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
            clipboard
                .set_text(text)
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
            Ok(())
        });

        methods.add_method("get", |lua, _, _: ()| {
            let mut clipboard =
                arboard::Clipboard::new().map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
            let content = clipboard
                .get_text()
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
            lua.create_string(&content)
        });

        methods.add_method("get_image", |lua, _, _: ()| {
            let mut clipboard =
                arboard::Clipboard::new().map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
            let image = clipboard
                .get_image()
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

            let img_data = image.bytes.into_owned();

            let table = lua.create_table()?;
            table.set("width", image.width)?;
            table.set("height", image.height)?;
            table.set("bytes", img_data)?;

            Ok(table)
        });
    }
}

fn find_script(program_name: &str) -> Option<PathBuf> {
    let script_name = format!("{}.{}.lua", program_name, env!("CARGO_PKG_NAME"));

    // check current directory
    let local_path = Path::new(&script_name);
    if local_path.exists() {
        return Some(local_path.to_path_buf());
    }

    // check user scripts directory
    if let Some(proj_dirs) = ProjectDirs::from("org", "winlogon", "lunash") {
        let user_script = proj_dirs
            .data_local_dir()
            .join("scripts")
            .join(&script_name);
        if user_script.exists() {
            return Some(user_script);
        }
    }

    // check PATH-like environment variable
    if let Ok(path_var) = env::var("LUA_SCRIPT_PATH") {
        for path in path_var.split(':') {
            let script_path = Path::new(path).join(&script_name);
            if script_path.exists() {
                return Some(script_path);
            }
        }
    }

    None
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run { name: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let args: Vec<String> = env::args().collect();

    match &cli.command {
        Commands::Run { name } => {
            let program_name = name;
            let script_path = find_script(program_name)
                .ok_or_else(|| format!("Script for '{}' not found", program_name))?;

            let http_client = Arc::new(Mutex::new(Client::new()));

            let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

            lua.set_app_data(Arc::clone(&http_client));

            let globals = lua.globals();
            globals.set("fs", FsUtils)?;
            globals.set("stringx", StringUtils)?;
            globals.set("clipboard", ClipboardHandling)?;

            // Add regex module with constructor
            let _ = globals.set(
                "regex",
                lua.create_function(|_, pattern: String| {
                    Regex::new(&pattern)
                        .map(RegexWrapper)
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))
                })?,
            );

            globals.set(
                "http",
                HttpModule {
                    client: Arc::clone(&http_client),
                },
            )?;

            let arg_table = lua.create_table()?;
            for (i, arg) in args.iter().enumerate() {
                arg_table.set(i + 1, arg.as_str())?;
            }
            globals.set("arg", arg_table)?;

            let script = fs::read_to_string(&script_path)
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

            lua.load(&script)
                .set_name(script_path.to_string_lossy().to_string())
                .exec()?;
        }
    }

    Ok(())
}

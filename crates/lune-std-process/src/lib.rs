#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::manual_let_else)]
use std::{
    env::consts::{ARCH, OS},
    fs,
    path::{MAIN_SEPARATOR, PathBuf},
    process::Stdio,
};

use mlua::prelude::*;
use mlua_luau_scheduler::Functions;

use lune_utils::{
    TableBuilder,
    path::get_current_dir,
    process::{ProcessArgs, ProcessEnv},
};

mod create;
mod exec;
mod options;

use self::options::ProcessSpawnOptions;

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}

fn load_dotenv_into_table(_: &Lua, env_table: &LuaTable) -> LuaResult<()> {
    let cwd = get_current_dir();
    let dotenv_path: PathBuf = cwd.join(".env");

    if !dotenv_path.exists() {
        return Ok(());
    }

    let contents = match fs::read_to_string(&dotenv_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    for line in contents.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            if !key.is_empty() {
                env_table.set(key, value)?;
            }
        }
    }

    Ok(())
}

/**
    Creates the `process` standard library module.

    # Errors

    Errors when out of memory.
*/
#[allow(clippy::missing_panics_doc)]
pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let mut cwd_str = get_current_dir()
        .to_str()
        .expect("cwd should be valid UTF-8")
        .to_string();

    if !cwd_str.ends_with(MAIN_SEPARATOR) {
        cwd_str.push(MAIN_SEPARATOR);
    }

    let os = lua.create_string(OS.to_lowercase())?;
    let arch = lua.create_string(ARCH.to_lowercase())?;
    let endianness = lua.create_string(if cfg!(target_endian = "big") {
        "big"
    } else {
        "little"
    })?;

    let process_args = lua
        .app_data_ref::<ProcessArgs>()
        .ok_or_else(|| LuaError::runtime("Missing process args in Lua app data"))?
        .into_plain_lua_table(lua.clone())?;

    let process_env = lua
        .app_data_ref::<ProcessEnv>()
        .ok_or_else(|| LuaError::runtime("Missing process env in Lua app data"))?
        .into_plain_lua_table(lua.clone())?;

    load_dotenv_into_table(&lua, &process_env)?;

    process_args.set_readonly(true);

    let fns = Functions::new(lua.clone())?;
    let process_exit = fns.exit;

    TableBuilder::new(lua)?
        .with_value("os", os)?
        .with_value("arch", arch)?
        .with_value("endianness", endianness)?
        .with_value("args", process_args)?
        .with_value("cwd", cwd_str)?
        .with_value("env", process_env)?
        .with_value("exit", process_exit)?
        .with_async_function("exec", process_exec)?
        .with_function("create", process_create)?
        .build_readonly()
}

async fn process_exec(
    lua: Lua,
    (program, args, mut options): (String, ProcessArgs, ProcessSpawnOptions),
) -> LuaResult<LuaTable> {
    let stdin = options.stdio.stdin.take();
    let stdout = options.stdio.stdout;
    let stderr = options.stdio.stderr;

    let stdin_stdio = if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };

    let child = options
        .into_command(program, args)
        .stdin(stdin_stdio)
        .stdout(stdout.as_stdio())
        .stderr(stderr.as_stdio())
        .spawn()?;

    exec::exec(lua, child, stdin, stdout, stderr).await
}

fn process_create(
    lua: &Lua,
    (program, args, options): (String, ProcessArgs, ProcessSpawnOptions),
) -> LuaResult<LuaValue> {
    let child = options
        .into_command(program, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    create::Child::new(lua, child).into_lua(lua)
}

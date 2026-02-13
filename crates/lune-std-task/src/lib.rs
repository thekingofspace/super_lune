#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::pedantic)]

use std::thread;
use std::time::{Duration, Instant};

use async_channel::{Receiver, Sender};
use async_io::Timer;
use futures_lite::future::yield_now;

use mlua::prelude::*;
use mlua_luau_scheduler::Functions;

use lune_utils::TableBuilder;

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}

#[derive(Clone, Debug)]
enum ThreadValue {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
    Table(Vec<(ThreadValue, ThreadValue)>),
}

fn to_thread_value(lua: &Lua, value: LuaValue) -> LuaResult<ThreadValue> {
    match value {
        LuaValue::Nil => Ok(ThreadValue::Nil),
        LuaValue::Boolean(b) => Ok(ThreadValue::Bool(b)),
        LuaValue::Integer(i) => Ok(ThreadValue::Number(i as f64)),
        LuaValue::Number(n) => Ok(ThreadValue::Number(n)),
        LuaValue::String(s) => Ok(ThreadValue::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => {
            let mut entries = Vec::new();
            for pair in t.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair?;
                entries.push((to_thread_value(lua, k)?, to_thread_value(lua, v)?));
            }
            Ok(ThreadValue::Table(entries))
        }
        _ => Err(LuaError::external("unsupported type for threading")),
    }
}

fn from_thread_value(lua: &Lua, value: ThreadValue) -> LuaResult<LuaValue> {
    match value {
        ThreadValue::Nil => Ok(LuaValue::Nil),
        ThreadValue::Bool(b) => Ok(LuaValue::Boolean(b)),
        ThreadValue::Number(n) => Ok(LuaValue::Number(n)),
        ThreadValue::String(s) => Ok(LuaValue::String(lua.create_string(&s)?)),
        ThreadValue::Table(entries) => {
            let table = lua.create_table()?;
            for (k, v) in entries {
                table.set(from_thread_value(lua, k)?, from_thread_value(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

struct ParallelTask {
    tx: Sender<Vec<ThreadValue>>,
    rx: Receiver<Vec<ThreadValue>>,
}

impl LuaUserData for ParallelTask {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("Push", |lua, this, args: LuaMultiValue| {
            let mut converted = Vec::new();
            for value in args {
                converted.push(to_thread_value(lua, value)?);
            }

            this.tx
                .send_blocking(converted)
                .map_err(|_| LuaError::external("failed to send"))?;
            Ok(())
        });

        methods.add_method("Pop", |lua, this, ()| {
            let values = this
                .rx
                .recv_blocking()
                .map_err(|_| LuaError::external("channel closed"))?;

            let mut result = Vec::new();
            for value in values {
                result.push(from_thread_value(lua, value)?);
            }

            Ok(LuaMultiValue::from_vec(result))
        });

        methods.add_method("Close", |_, this, ()| {
            this.tx.close();
            Ok(())
        });
    }
}

fn install_worker_api(
    lua: &Lua,
    tx_out: Sender<Vec<ThreadValue>>,
    rx_in: Receiver<Vec<ThreadValue>>,
) -> LuaResult<()> {
    let globals = lua.globals();
    let task = lua.create_table()?;

    task.set(
        "pop",
        lua.create_function(move |lua, ()| {
            let values = match rx_in.recv_blocking() {
                Ok(v) => v,
                Err(_) => {
                    return Ok(LuaMultiValue::from_vec(vec![LuaValue::Nil]));
                }
            };

            let mut result = Vec::new();
            for value in values {
                result.push(from_thread_value(lua, value)?);
            }

            Ok(LuaMultiValue::from_vec(result))
        })?,
    )?;

    task.set(
        "push",
        lua.create_function(move |lua, args: LuaMultiValue| {
            let mut converted = Vec::new();
            for value in args {
                converted.push(to_thread_value(&lua, value)?);
            }

            if tx_out.send_blocking(converted).is_err() {
                return Ok(());
            }

            Ok(())
        })?,
    )?;

    globals.set("task", task)?;
    Ok(())
}

fn parallel(lua: &Lua, script: String) -> LuaResult<LuaAnyUserData> {
    let (tx_in, rx_in) = async_channel::unbounded::<Vec<ThreadValue>>();
    let (tx_out, rx_out) = async_channel::unbounded::<Vec<ThreadValue>>();

    thread::spawn(move || {
        let worker_lua = Lua::new();

        install_worker_api(&worker_lua, tx_out.clone(), rx_in.clone())
            .expect("failed to install worker api");

        if let Err(err) = worker_lua.load(&script).exec() {
            eprintln!("Worker script error: {err}");
        }
    });

    lua.create_userdata(ParallelTask {
        tx: tx_in,
        rx: rx_out,
    })
}
pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let fns = Functions::new(lua.clone())?;

    let task_wait = lua.create_async_function(wait)?;

    let task_delay_env = TableBuilder::new(lua.clone())?
        .with_value("select", lua.globals().get::<LuaFunction>("select")?)?
        .with_value("spawn", fns.spawn.clone())?
        .with_value("defer", fns.defer.clone())?
        .with_value("wait", task_wait.clone())?
        .build_readonly()?;

    let task_delay = lua
        .load(DELAY_IMPL_LUA)
        .set_name("task.delay")
        .set_environment(task_delay_env)
        .into_function()?;

    let task_parallel = lua.create_function(|lua, script: String| parallel(&lua, script))?;

    TableBuilder::new(lua)?
        .with_value("cancel", fns.cancel)?
        .with_value("defer", fns.defer)?
        .with_value("delay", task_delay)?
        .with_value("spawn", fns.spawn)?
        .with_value("wait", task_wait)?
        .with_value("parallel", task_parallel)?
        .build_readonly()
}

const DELAY_IMPL_LUA: &str = r"
return defer(function(...)
    wait(select(1, ...))
    spawn(select(2, ...))
end, ...)
";

async fn wait(lua: Lua, secs: Option<f64>) -> LuaResult<f64> {
    yield_now().await;
    wait_inner(lua, secs).await
}

async fn wait_inner(_: Lua, secs: Option<f64>) -> LuaResult<f64> {
    let duration = Duration::from_secs_f64(secs.unwrap_or_default());
    let duration = duration.max(Duration::from_millis(1));

    yield_now().await;

    let before = Instant::now();
    let after = Timer::after(duration).await;

    Ok((after - before).as_secs_f64())
}

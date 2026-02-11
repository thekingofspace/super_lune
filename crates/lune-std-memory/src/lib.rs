#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::pedantic)]

use std::{
    cell::RefCell,
    collections::HashSet,
    mem::size_of,
    rc::Rc,
    time::{Duration, Instant},
};

use lune_utils::TableBuilder;
use mlua::prelude::*;

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}

#[derive(Clone)]
struct MemoryBlock {
    inner: Rc<RefCell<Inner>>,
}

struct Inner {
    capacity: usize,
    buffer: Vec<LuaValue>,
    scheduled: Option<Instant>,
    freed: bool,
}

impl MemoryBlock {
    fn new(capacity: usize) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                capacity,
                buffer: Vec::new(),
                scheduled: None,
                freed: false,
            })),
        }
    }

    fn check_alive(inner: &Inner) -> LuaResult<()> {
        if inner.freed {
            Err(LuaError::runtime("Memory block already freed"))
        } else {
            Ok(())
        }
    }

    fn validate_value(value: &LuaValue) -> LuaResult<()> {
        match value {
            LuaValue::Nil
            | LuaValue::Boolean(_)
            | LuaValue::Integer(_)
            | LuaValue::Number(_)
            | LuaValue::String(_) => Ok(()),

            LuaValue::Table(t) => {
                if t.metatable().is_some() {
                    return Err(LuaError::runtime(
                        "Metatables are not allowed in MemoryBlock",
                    ));
                }

                for pair in t.clone().pairs::<LuaValue, LuaValue>() {
                    let (k, v) = pair?;
                    Self::validate_value(&k)?;
                    Self::validate_value(&v)?;
                }

                Ok(())
            }

            _ => Err(LuaError::runtime(
                "Unsupported value type (only bool, number, string, table allowed)",
            )),
        }
    }

    fn value_size(value: &LuaValue, visited: &mut HashSet<usize>) -> LuaResult<usize> {
        Ok(match value {
            LuaValue::Nil => 0,

            LuaValue::Boolean(_) => size_of::<bool>(),

            LuaValue::Integer(_) => size_of::<i64>(),

            LuaValue::Number(_) => size_of::<f64>(),

            LuaValue::String(s) => size_of::<LuaValue>() + s.as_bytes().len(),

            LuaValue::Table(t) => {
                let ptr = t.to_pointer() as usize;

                if !visited.insert(ptr) {
                    return Ok(0);
                }

                let mut total = size_of::<LuaValue>();

                for pair in t.clone().pairs::<LuaValue, LuaValue>() {
                    let (k, v) = pair?;
                    total += Self::value_size(&k, visited)?;
                    total += Self::value_size(&v, visited)?;
                }

                total
            }

            _ => 0,
        })
    }

    fn total_size(inner: &Inner) -> LuaResult<usize> {
        let mut visited = HashSet::new();

        let mut total = 0;
        for value in &inner.buffer {
            total += Self::value_size(value, &mut visited)?;
        }

        for value in &inner.buffer {
            total += Self::value_size(value, &mut visited)?;
        }

        Ok(total)
    }
}

impl LuaUserData for MemoryBlock {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("Write", |_, this, value: LuaValue| {
            let mut inner = this.inner.borrow_mut();
            Self::check_alive(&inner)?;
            Self::validate_value(&value)?;

            inner.buffer.push(value);

            let used = Self::total_size(&inner)?;

            if used > inner.capacity {
                inner.buffer.pop();
                return Err(LuaError::runtime("Fatal: memory exceeded capacity"));
            }

            Ok(())
        });

        methods.add_method("Read", |lua, this, ()| {
            let inner = this.inner.borrow();
            Self::check_alive(&inner)?;

            match inner.buffer.len() {
                0 => Ok(LuaValue::Nil),
                1 => Ok(inner.buffer[0].clone()),
                _ => {
                    let table = lua.create_table()?;
                    for (i, value) in inner.buffer.iter().enumerate() {
                        table.set(i + 1, value.clone())?;
                    }
                    Ok(LuaValue::Table(table))
                }
            }
        });

        methods.add_method_mut("Free", |_, this, ()| {
            let mut inner = this.inner.borrow_mut();
            inner.buffer.clear();
            inner.freed = true;
            inner.scheduled = None;
            Ok(())
        });

        methods.add_method_mut("Schedule", |_, this, ttl: Option<f64>| {
            let mut inner = this.inner.borrow_mut();
            Self::check_alive(&inner)?;

            let duration = ttl.unwrap_or(60.0);
            inner.scheduled = Some(Instant::now() + Duration::from_secs_f64(duration));

            Ok(())
        });

        methods.add_method("Size", |_, this, ()| {
            let inner = this.inner.borrow();
            Self::check_alive(&inner)?;
            Self::total_size(&inner)
        });

        methods.add_method("Capacity", |_, this, ()| {
            let inner = this.inner.borrow();
            Ok(inner.capacity)
        });
    }
}

#[derive(Default)]
struct MemoryRegistry {
    blocks: RefCell<Vec<MemoryBlock>>,
}

impl MemoryRegistry {
    fn new() -> Self {
        Self {
            blocks: RefCell::new(Vec::new()),
        }
    }
}

pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let registry = Rc::new(MemoryRegistry::new());

    let malloc_registry = registry.clone();
    let clean_registry = registry.clone();

    TableBuilder::new(lua.clone())?
        .with_function("malloc", move |_, size: usize| {
            if size == 0 {
                return Err(LuaError::runtime("Cannot allocate zero-sized memory block"));
            }

            let block = MemoryBlock::new(size);
            malloc_registry.blocks.borrow_mut().push(block.clone());

            Ok(block)
        })?
        .with_function("Clean", move |_, callback: LuaFunction| {
            let mut blocks = clean_registry.blocks.borrow_mut();
            let now = Instant::now();

            blocks.retain(|block| {
                let mut inner = block.inner.borrow_mut();

                if inner.freed {
                    return false;
                }

                let expired = inner.scheduled.map(|t| now >= t).unwrap_or(false);

                let should_clean: bool = callback.call(block.clone()).unwrap_or(false);

                if expired || should_clean {
                    inner.buffer.clear();
                    inner.freed = true;
                    inner.scheduled = None;
                    return false;
                }

                true
            });

            Ok(())
        })?
        .build_readonly()
}

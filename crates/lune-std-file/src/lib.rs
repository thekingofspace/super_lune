#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::missing_errors_doc)]

use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lune_utils::TableBuilder;

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

const TYPE_NIL: u8 = 1;
const TYPE_BOOL: u8 = 2;
const TYPE_NUMBER: u8 = 3;
const TYPE_STRING: u8 = 4;
const TYPE_INTEGER: u8 = 5;
const TYPE_TABLE: u8 = 6;

#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}

#[derive(Clone)]
struct FileObject {
    raw_region: Arc<Mutex<Vec<u8>>>,
    safe_region: Arc<Mutex<HashMap<u32, Vec<u8>>>>,
}

impl FileObject {
    fn new() -> Self {
        Self {
            raw_region: Arc::new(Mutex::new(Vec::new())),
            safe_region: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let raw = self.raw_region.lock().unwrap();
        let safe = self.safe_region.lock().unwrap();

        let mut out = Vec::new();
        out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        out.extend_from_slice(&raw);

        out.extend_from_slice(&(safe.len() as u32).to_le_bytes());

        for (slot, data) in safe.iter() {
            out.extend_from_slice(&slot.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(data);
        }

        out
    }

    fn deserialize(bytes: Vec<u8>) -> Self {
        let mut cursor = 0;

        if bytes.len() < 4 {
            return Self::new();
        }

        // RAW
        let mut raw_len_arr = [0u8; 4];
        raw_len_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
        cursor += 4;

        let raw_len = u32::from_le_bytes(raw_len_arr) as usize;

        if cursor + raw_len > bytes.len() {
            return Self::new();
        }

        let raw_region = bytes[cursor..cursor + raw_len].to_vec();
        cursor += raw_len;

        let mut safe_region = HashMap::new();

        if cursor + 4 <= bytes.len() {
            let mut count_arr = [0u8; 4];
            count_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
            cursor += 4;

            let count = u32::from_le_bytes(count_arr);

            for _ in 0..count {
                if cursor + 8 > bytes.len() {
                    break;
                }

                let mut slot_arr = [0u8; 4];
                slot_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
                cursor += 4;

                let slot = u32::from_le_bytes(slot_arr);

                let mut len_arr = [0u8; 4];
                len_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
                cursor += 4;

                let len = u32::from_le_bytes(len_arr) as usize;

                if cursor + len > bytes.len() {
                    break;
                }

                let data = bytes[cursor..cursor + len].to_vec();
                cursor += len;

                safe_region.insert(slot, data);
            }
        }

        Self {
            raw_region: Arc::new(Mutex::new(raw_region)),
            safe_region: Arc::new(Mutex::new(safe_region)),
        }
    }

    fn encode_value(lua: &Lua, value: LuaValue, out: &mut Vec<u8>) -> LuaResult<()> {
        match value {
            LuaValue::Nil => out.push(TYPE_NIL),

            LuaValue::Boolean(b) => {
                out.push(TYPE_BOOL);
                out.push(u8::from(b));
            }

            LuaValue::Number(n) => {
                out.push(TYPE_NUMBER);
                out.extend_from_slice(&n.to_le_bytes());
            }

            LuaValue::Integer(i) => {
                out.push(TYPE_INTEGER);
                out.extend_from_slice(&i.to_le_bytes());
            }

            LuaValue::String(s) => {
                out.push(TYPE_STRING);
                let bytes = s.as_bytes();
                let len = bytes.len() as u32;
                out.extend_from_slice(&len.to_le_bytes());
                out.extend_from_slice(bytes.as_ref());
            }

            LuaValue::Table(t) => {
                out.push(TYPE_TABLE);

                let mut pairs = Vec::new();
                for pair in t.pairs::<LuaValue, LuaValue>() {
                    pairs.push(pair?);
                }

                out.extend_from_slice(&(pairs.len() as u32).to_le_bytes());

                for (k, v) in pairs {
                    Self::encode_value(lua, k, out)?;
                    Self::encode_value(lua, v, out)?;
                }
            }

            _ => return Err(LuaError::external("Unsupported Lua type")),
        }

        Ok(())
    }

    fn decode_value(lua: &Lua, buffer: &[u8]) -> LuaResult<LuaValue> {
        let mut cursor = 0;
        Self::decode_internal(lua, buffer, &mut cursor)
    }

    fn decode_internal(lua: &Lua, buffer: &[u8], cursor: &mut usize) -> LuaResult<LuaValue> {
        if *cursor >= buffer.len() {
            return Ok(LuaValue::Nil);
        }

        let tag = buffer[*cursor];
        *cursor += 1;

        match tag {
            TYPE_NIL => Ok(LuaValue::Nil),

            TYPE_BOOL => {
                let b = buffer[*cursor] == 1;
                *cursor += 1;
                Ok(LuaValue::Boolean(b))
            }

            TYPE_NUMBER => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&buffer[*cursor..*cursor + 8]);
                *cursor += 8;
                Ok(LuaValue::Number(f64::from_le_bytes(arr)))
            }

            TYPE_INTEGER => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&buffer[*cursor..*cursor + 8]);
                *cursor += 8;
                Ok(LuaValue::Integer(i64::from_le_bytes(arr)))
            }

            TYPE_STRING => {
                let mut len_arr = [0u8; 4];
                len_arr.copy_from_slice(&buffer[*cursor..*cursor + 4]);
                *cursor += 4;

                let len = u32::from_le_bytes(len_arr) as usize;
                let data = &buffer[*cursor..*cursor + len];
                *cursor += len;

                Ok(LuaValue::String(lua.create_string(data)?))
            }

            TYPE_TABLE => {
                let mut len_arr = [0u8; 4];
                len_arr.copy_from_slice(&buffer[*cursor..*cursor + 4]);
                *cursor += 4;

                let count = u32::from_le_bytes(len_arr);

                let table = lua.create_table()?;

                for _ in 0..count {
                    let key = Self::decode_internal(lua, buffer, cursor)?;
                    let value = Self::decode_internal(lua, buffer, cursor)?;
                    table.set(key, value)?;
                }

                Ok(LuaValue::Table(table))
            }

            _ => Err(LuaError::external("Invalid data")),
        }
    }

    fn write(&self, lua: &Lua, pos: usize, value: LuaValue) -> LuaResult<()> {
        let mut raw = self.raw_region.lock().unwrap();

        let mut bytes = Vec::new();
        Self::encode_value(lua, value, &mut bytes)?;

        if raw.len() < pos {
            raw.resize(pos, 0);
        }

        if raw.len() < pos + bytes.len() {
            raw.resize(pos + bytes.len(), 0);
        }

        raw[pos..pos + bytes.len()].copy_from_slice(&bytes);

        Ok(())
    }

    fn read(&self, lua: &Lua, pos: usize) -> LuaResult<LuaValue> {
        let raw = self.raw_region.lock().unwrap();
        if pos >= raw.len() {
            return Ok(LuaValue::Nil);
        }

        let slice = &raw[pos..];
        Self::decode_value(lua, slice)
    }

    fn safe_write(&self, lua: &Lua, slot: u32, value: LuaValue) -> LuaResult<()> {
        let mut safe = self.safe_region.lock().unwrap();

        let mut bytes = Vec::new();
        Self::encode_value(lua, value, &mut bytes)?;

        safe.insert(slot, bytes);

        Ok(())
    }

    fn safe_read(&self, lua: &Lua, slot: u32) -> LuaResult<LuaValue> {
        let safe = self.safe_region.lock().unwrap();

        if let Some(bytes) = safe.get(&slot) {
            return Self::decode_value(lua, bytes);
        }

        Ok(LuaValue::Nil)
    }
}

impl LuaUserData for FileObject {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("write", |lua, this, (pos, value): (usize, LuaValue)| {
            this.write(lua, pos, value)
        });

        methods.add_method("read", |lua, this, pos: usize| this.read(lua, pos));

        methods.add_method("safeWrite", |lua, this, (slot, value): (u32, LuaValue)| {
            this.safe_write(lua, slot, value)
        });

        methods.add_method("safeRead", |lua, this, slot: u32| this.safe_read(lua, slot));

        methods.add_method("serialize", |lua, this, ()| {
            let bytes = this.serialize();
            Ok(lua.create_string(&bytes))
        });
    }
}

pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    TableBuilder::new(lua)?
        .with_function("new", |_, ()| Ok(FileObject::new()))?
        .with_function("deserialize", |_, bytes: LuaString| {
            Ok(FileObject::deserialize(bytes.as_bytes().as_ref().to_vec()))
        })?
        .build_readonly()
}

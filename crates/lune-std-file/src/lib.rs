#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::needless_question_mark)]
#![allow(clippy::needless_borrows_for_generic_args)]

use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lune_utils::TableBuilder;

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}

const TYPE_I8: u8 = 1;
const TYPE_U8: u8 = 2;
const TYPE_I16: u8 = 3;
const TYPE_U16: u8 = 4;
const TYPE_I32: u8 = 5;
const TYPE_U32: u8 = 6;
const TYPE_I64: u8 = 7;
const TYPE_U64: u8 = 8;
const TYPE_F32: u8 = 9;
const TYPE_F64: u8 = 10;
const TYPE_BOOL: u8 = 11;
const TYPE_STRING: u8 = 12;

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

    fn write_typed(&self, lua: &Lua, pos: usize, type_id: u8, value: LuaValue) -> LuaResult<()> {
        let mut raw = self.raw_region.lock().unwrap();

        let mut bytes = Vec::new();
        bytes.push(type_id);

        match type_id {
            TYPE_I8 => bytes.push(lua.unpack::<i8>(value)? as u8),
            TYPE_U8 => bytes.push(lua.unpack::<u8>(value)?),
            TYPE_I16 => bytes.extend_from_slice(&lua.unpack::<i16>(value)?.to_le_bytes()),
            TYPE_U16 => bytes.extend_from_slice(&lua.unpack::<u16>(value)?.to_le_bytes()),
            TYPE_I32 => bytes.extend_from_slice(&lua.unpack::<i32>(value)?.to_le_bytes()),
            TYPE_U32 => bytes.extend_from_slice(&lua.unpack::<u32>(value)?.to_le_bytes()),
            TYPE_I64 => bytes.extend_from_slice(&lua.unpack::<i64>(value)?.to_le_bytes()),
            TYPE_U64 => bytes.extend_from_slice(&lua.unpack::<u64>(value)?.to_le_bytes()),
            TYPE_F32 => bytes.extend_from_slice(&lua.unpack::<f32>(value)?.to_le_bytes()),
            TYPE_F64 => bytes.extend_from_slice(&lua.unpack::<f64>(value)?.to_le_bytes()),
            TYPE_BOOL => bytes.push(u8::from(lua.unpack::<bool>(value)?)),
            TYPE_STRING => {
                let s: LuaString = lua.unpack(value)?;
                let b = s.as_bytes();
                let len = b.len() as u32;
                bytes.extend_from_slice(&len.to_le_bytes());
                bytes.extend_from_slice(b.as_ref());
            }
            _ => return Err(LuaError::external("Invalid type id")),
        }

        if raw.len() < pos {
            raw.resize(pos, 0);
        }

        if raw.len() < pos + bytes.len() {
            raw.resize(pos + bytes.len(), 0);
        }

        raw[pos..pos + bytes.len()].copy_from_slice(&bytes);

        Ok(())
    }

    fn read_typed(&self, lua: &Lua, pos: usize) -> LuaResult<LuaValue> {
        let raw = self.raw_region.lock().unwrap();

        if pos >= raw.len() {
            return Ok(LuaValue::Nil);
        }

        let mut cursor = pos;
        let type_id = raw[cursor];
        cursor += 1;

        let value = match type_id {
            TYPE_I8 => LuaValue::Integer(raw[cursor] as i8 as i64),
            TYPE_U8 => LuaValue::Integer(raw[cursor] as i64),
            TYPE_I16 => {
                let mut arr = [0u8; 2];
                arr.copy_from_slice(&raw[cursor..cursor + 2]);
                LuaValue::Integer(i16::from_le_bytes(arr) as i64)
            }
            TYPE_U16 => {
                let mut arr = [0u8; 2];
                arr.copy_from_slice(&raw[cursor..cursor + 2]);
                LuaValue::Integer(u16::from_le_bytes(arr) as i64)
            }
            TYPE_I32 => {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&raw[cursor..cursor + 4]);
                LuaValue::Integer(i32::from_le_bytes(arr) as i64)
            }
            TYPE_U32 => {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&raw[cursor..cursor + 4]);
                LuaValue::Integer(u32::from_le_bytes(arr) as i64)
            }
            TYPE_I64 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&raw[cursor..cursor + 8]);
                LuaValue::Integer(i64::from_le_bytes(arr))
            }
            TYPE_U64 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&raw[cursor..cursor + 8]);
                LuaValue::Integer(u64::from_le_bytes(arr) as i64)
            }
            TYPE_F32 => {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&raw[cursor..cursor + 4]);
                LuaValue::Number(f32::from_le_bytes(arr) as f64)
            }
            TYPE_F64 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&raw[cursor..cursor + 8]);
                LuaValue::Number(f64::from_le_bytes(arr))
            }
            TYPE_BOOL => LuaValue::Boolean(raw[cursor] == 1),
            TYPE_STRING => {
                let mut len_arr = [0u8; 4];
                len_arr.copy_from_slice(&raw[cursor..cursor + 4]);
                cursor += 4;
                let len = u32::from_le_bytes(len_arr) as usize;
                let data = &raw[cursor..cursor + len];
                LuaValue::String(lua.create_string(data)?)
            }
            _ => return Err(LuaError::external("Invalid type id")),
        };

        Ok(value)
    }

    fn safe_write(&self, lua: &Lua, slot: u32, value: LuaValue) -> LuaResult<()> {
        let mut safe = self.safe_region.lock().unwrap();

        let mut bytes = Vec::new();
        Self::encode_safe_value(lua, value, &mut bytes)?;
        safe.insert(slot, bytes);

        Ok(())
    }

    fn safe_read(&self, lua: &Lua, slot: u32) -> LuaResult<LuaValue> {
        let safe = self.safe_region.lock().unwrap();

        if let Some(bytes) = safe.get(&slot) {
            Self::decode_safe_value(lua, bytes)
        } else {
            Ok(LuaValue::Nil)
        }
    }

    fn encode_safe_value(_: &Lua, value: LuaValue, out: &mut Vec<u8>) -> LuaResult<()> {
        match value {
            LuaValue::Nil => out.push(0),
            LuaValue::Boolean(b) => {
                out.push(1);
                out.push(u8::from(b));
            }
            LuaValue::Integer(i) => {
                out.push(2);
                out.extend_from_slice(&i.to_le_bytes());
            }
            LuaValue::Number(n) => {
                out.push(3);
                out.extend_from_slice(&n.to_le_bytes());
            }
            LuaValue::String(s) => {
                out.push(4);
                let b = s.as_bytes();
                let len = b.len() as u32;
                out.extend_from_slice(&len.to_le_bytes());
                out.extend_from_slice(b.as_ref());
            }
            _ => return Err(LuaError::external("Unsupported safe type")),
        }
        Ok(())
    }

    fn decode_safe_value(lua: &Lua, buffer: &[u8]) -> LuaResult<LuaValue> {
        if buffer.is_empty() {
            return Ok(LuaValue::Nil);
        }

        let mut cursor = 0;
        let tag = buffer[cursor];
        cursor += 1;

        match tag {
            0 => Ok(LuaValue::Nil),
            1 => Ok(LuaValue::Boolean(buffer[cursor] == 1)),
            2 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&buffer[cursor..cursor + 8]);
                Ok(LuaValue::Integer(i64::from_le_bytes(arr)))
            }
            3 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&buffer[cursor..cursor + 8]);
                Ok(LuaValue::Number(f64::from_le_bytes(arr)))
            }
            4 => {
                let mut len_arr = [0u8; 4];
                len_arr.copy_from_slice(&buffer[cursor..cursor + 4]);
                cursor += 4;
                let len = u32::from_le_bytes(len_arr) as usize;
                let data = &buffer[cursor..cursor + len];
                Ok(LuaValue::String(lua.create_string(data)?))
            }
            _ => Err(LuaError::external("Invalid safe data")),
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

        let mut raw_len_arr = [0u8; 4];
        raw_len_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
        cursor += 4;

        let raw_len = u32::from_le_bytes(raw_len_arr) as usize;
        let raw_region = bytes[cursor..cursor + raw_len].to_vec();
        cursor += raw_len;

        let mut safe_region = HashMap::new();

        if cursor + 4 <= bytes.len() {
            let mut count_arr = [0u8; 4];
            count_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
            cursor += 4;
            let count = u32::from_le_bytes(count_arr);

            for _ in 0..count {
                let mut slot_arr = [0u8; 4];
                slot_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
                cursor += 4;
                let slot = u32::from_le_bytes(slot_arr);

                let mut len_arr = [0u8; 4];
                len_arr.copy_from_slice(&bytes[cursor..cursor + 4]);
                cursor += 4;
                let len = u32::from_le_bytes(len_arr) as usize;

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
}

impl LuaUserData for FileObject {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "write",
            |lua, this, (pos, type_id, value): (usize, u8, LuaValue)| {
                this.write_typed(lua, pos, type_id, value)
            },
        );

        methods.add_method("read", |lua, this, pos: usize| this.read_typed(lua, pos));

        methods.add_method("safeWrite", |lua, this, (slot, value): (u32, LuaValue)| {
            this.safe_write(lua, slot, value)
        });

        methods.add_method("safeRead", |lua, this, slot: u32| this.safe_read(lua, slot));

        methods.add_method("serialize", |lua, this, ()| {
            Ok(lua.create_string(&this.serialize())?)
        });
    }
}

pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let types = lua.create_table()?;

    types.set("i8", TYPE_I8)?;
    types.set("u8", TYPE_U8)?;
    types.set("i16", TYPE_I16)?;
    types.set("u16", TYPE_U16)?;
    types.set("i32", TYPE_I32)?;
    types.set("u32", TYPE_U32)?;
    types.set("i64", TYPE_I64)?;
    types.set("u64", TYPE_U64)?;
    types.set("f32", TYPE_F32)?;
    types.set("f64", TYPE_F64)?;
    types.set("bool", TYPE_BOOL)?;
    types.set("string", TYPE_STRING)?;

    TableBuilder::new(lua)?
        .with_function("new", |_, ()| Ok(FileObject::new()))?
        .with_function("deserialize", |_, bytes: LuaString| {
            Ok(FileObject::deserialize(bytes.as_bytes().as_ref().to_vec()))
        })?
        .with_value("types", types)?
        .build_readonly()
}

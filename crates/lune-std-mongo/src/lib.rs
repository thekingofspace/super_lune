#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]

use futures::StreamExt;
use lune_utils::TableBuilder;
use mlua::{UserData, UserDataMethods, prelude::*};
use mongodb::{
    Client,
    bson::{Bson, DateTime, Document, oid::ObjectId},
};
use std::sync::{Arc, LazyLock};
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: LazyLock<Runtime> =
    LazyLock::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}

pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let object_api = create_object_api(&lua)?;

    TableBuilder::new(lua)?
        .with_async_function("connect", mongo_connect)?
        .with_value("object", object_api)?
        .build_readonly()
}

#[derive(Clone)]
pub struct LuaObjectId {
    inner: ObjectId,
}

impl UserData for LuaObjectId {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("toHex", |_, this, ()| Ok(this.inner.to_hex()));
    }
}

#[derive(Clone)]
pub struct LuaDateTime {
    inner: DateTime,
}

impl UserData for LuaDateTime {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("toMillis", |_, this, ()| Ok(this.inner.timestamp_millis()));
    }
}

#[derive(Clone)]
pub struct LuaMongoClient {
    inner: Arc<Client>,
}

#[derive(Clone)]
pub struct LuaMongoDatabase {
    inner: mongodb::Database,
}

#[derive(Clone)]
pub struct LuaMongoCollection {
    inner: mongodb::Collection<Document>,
}

async fn mongo_connect(_: Lua, uri: String) -> LuaResult<LuaMongoClient> {
    let client = TOKIO_RUNTIME
        .block_on(async {
            let options = mongodb::options::ClientOptions::parse(uri).await?;
            Client::with_options(options)
        })
        .into_lua_err()?;

    Ok(LuaMongoClient {
        inner: Arc::new(client),
    })
}

impl UserData for LuaMongoClient {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("database", |_, this, name: String| {
            Ok(LuaMongoDatabase {
                inner: this.inner.database(&name),
            })
        });
    }
}

impl UserData for LuaMongoDatabase {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("collection", |_, this, name: String| {
            Ok(LuaMongoCollection {
                inner: this.inner.collection::<Document>(&name),
            })
        });
    }
}

impl UserData for LuaMongoCollection {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("insertOne", |lua, this, value: LuaValue| async move {
            let doc = lua_value_to_document(value.clone())?;

            let result = TOKIO_RUNTIME
                .block_on(async { this.inner.insert_one(doc).await })
                .into_lua_err()?;

            if let Some(id) = result.inserted_id.as_object_id() {
                let oid = LuaObjectId { inner: id };

                if let LuaValue::Table(table) = value {
                    table.set("_id", lua.create_userdata(oid.clone())?)?;
                }

                return Ok(LuaValue::UserData(lua.create_userdata(oid)?));
            }

            Ok(LuaValue::Nil)
        });

        methods.add_async_method(
            "findOne",
            |lua, this, (filter_value, options): (LuaValue, Option<LuaTable>)| async move {
                let filter = lua_value_to_document(filter_value)?;
                let mut query = this.inner.find_one(filter);

                if let Some(opt_table) = options {
                    if let Ok(sort) = opt_table.get::<LuaValue>("sort") {
                        let sort_doc = lua_value_to_document(sort)?;
                        query = query.sort(sort_doc);
                    }
                    if let Ok(projection) = opt_table.get::<LuaValue>("projection") {
                        let proj_doc = lua_value_to_document(projection)?;
                        query = query.projection(proj_doc);
                    }
                }

                let result = TOKIO_RUNTIME
                    .block_on(async { query.await })
                    .into_lua_err()?;

                match result {
                    Some(doc) => document_to_lua(lua, doc),
                    None => Ok(LuaValue::Nil),
                }
            },
        );

        methods.add_async_method(
            "find",
            |lua, this, (filter_value, options): (LuaValue, Option<LuaTable>)| async move {
                let filter = lua_value_to_document(filter_value)?;
                let mut query = this.inner.find(filter);

                if let Some(opt_table) = options {
                    if let Ok(sort) = opt_table.get::<LuaValue>("sort") {
                        let sort_doc = lua_value_to_document(sort)?;
                        query = query.sort(sort_doc);
                    }
                    if let Ok(limit) = opt_table.get::<i64>("limit") {
                        query = query.limit(limit);
                    }
                    if let Ok(skip) = opt_table.get::<u64>("skip") {
                        query = query.skip(skip);
                    }
                    if let Ok(projection) = opt_table.get::<LuaValue>("projection") {
                        let proj_doc = lua_value_to_document(projection)?;
                        query = query.projection(proj_doc);
                    }
                }

                let mut cursor = TOKIO_RUNTIME
                    .block_on(async { query.await })
                    .into_lua_err()?;

                let result_table = lua.create_table()?;
                let mut index = 1;

                while let Some(doc) = TOKIO_RUNTIME.block_on(async { cursor.next().await }) {
                    let doc = doc.into_lua_err()?;
                    result_table.set(index, document_to_lua(lua.clone(), doc)?)?;
                    index += 1;
                }

                Ok(result_table)
            },
        );

        methods.add_async_method(
            "updateOne",
            |_, this, (f, u, options): (LuaValue, LuaValue, Option<LuaTable>)| async move {
                let filter = lua_value_to_document(f)?;
                let update = lua_value_to_document(u)?;
                let mut query = this.inner.update_one(filter, update);

                if let Some(opt_table) = options {
                    if let Ok(upsert) = opt_table.get::<bool>("upsert") {
                        query = query.upsert(upsert);
                    }
                }

                TOKIO_RUNTIME
                    .block_on(async { query.await })
                    .into_lua_err()?;

                Ok(())
            },
        );

        methods.add_async_method(
            "updateMany",
            |_, this, (f, u, options): (LuaValue, LuaValue, Option<LuaTable>)| async move {
                let filter = lua_value_to_document(f)?;
                let update = lua_value_to_document(u)?;
                let mut query = this.inner.update_many(filter, update);

                if let Some(opt_table) = options {
                    if let Ok(upsert) = opt_table.get::<bool>("upsert") {
                        query = query.upsert(upsert);
                    }
                }

                TOKIO_RUNTIME
                    .block_on(async { query.await })
                    .into_lua_err()?;

                Ok(())
            },
        );

        methods.add_async_method("deleteOne", |_, this, filter| async move {
            let filter = lua_value_to_document(filter)?;
            TOKIO_RUNTIME
                .block_on(async { this.inner.delete_one(filter).await })
                .into_lua_err()?;
            Ok(())
        });

        methods.add_async_method("deleteMany", |_, this, filter| async move {
            let filter = lua_value_to_document(filter)?;
            TOKIO_RUNTIME
                .block_on(async { this.inner.delete_many(filter).await })
                .into_lua_err()?;
            Ok(())
        });

        methods.add_async_method("countDocuments", |_, this, filter| async move {
            let filter = lua_value_to_document(filter)?;
            TOKIO_RUNTIME
                .block_on(async { this.inner.count_documents(filter).await })
                .into_lua_err()
        });
    }
}

fn lua_value_to_document(value: LuaValue) -> LuaResult<Document> {
    match lua_to_bson(value)? {
        Bson::Document(doc) => Ok(doc),
        _ => Ok(Document::new()),
    }
}

fn lua_to_bson(value: LuaValue) -> LuaResult<Bson> {
    Ok(match value {
        LuaValue::Boolean(b) => Bson::Boolean(b),
        LuaValue::Integer(i) => Bson::Int64(i),
        LuaValue::Number(n) => Bson::Double(n),

        LuaValue::UserData(ud) => {
            if let Ok(oid) = ud.borrow::<LuaObjectId>() {
                Bson::ObjectId(oid.inner)
            } else if let Ok(dt) = ud.borrow::<LuaDateTime>() {
                Bson::DateTime(dt.inner)
            } else {
                Bson::Null
            }
        }

        LuaValue::String(s) => Bson::String(s.to_str()?.to_string()),

        LuaValue::Table(table) => {
            let mut doc = Document::new();

            for pair in table.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair?;

                if let LuaValue::String(key) = k
                    && let Ok(key) = key.to_str()
                {
                    doc.insert(key.to_string(), lua_to_bson(v)?);
                }
            }

            Bson::Document(doc)
        }

        _ => Bson::Null,
    })
}

fn document_to_lua(lua: Lua, doc: Document) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    for (k, v) in doc {
        table.set(k, bson_to_lua(lua.clone(), v)?)?;
    }
    Ok(LuaValue::Table(table))
}

fn bson_to_lua(lua: Lua, value: Bson) -> LuaResult<LuaValue> {
    Ok(match value {
        Bson::Boolean(b) => LuaValue::Boolean(b),
        Bson::Int32(i) => LuaValue::Integer(i as i64),
        Bson::Int64(i) => LuaValue::Integer(i),
        Bson::Double(f) => LuaValue::Number(f),
        Bson::String(s) => LuaValue::String(lua.create_string(&s)?),
        Bson::ObjectId(oid) => LuaValue::UserData(lua.create_userdata(LuaObjectId { inner: oid })?),
        Bson::DateTime(dt) => LuaValue::UserData(lua.create_userdata(LuaDateTime { inner: dt })?),
        Bson::Document(doc) => document_to_lua(lua, doc)?,
        _ => LuaValue::Nil,
    })
}

fn create_object_api(lua: &Lua) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;

    table.set(
        "objectId",
        lua.create_function(|lua, ()| {
            lua.create_userdata(LuaObjectId {
                inner: ObjectId::new(),
            })
        })?,
    )?;

    table.set(
        "date",
        lua.create_function(|lua, ()| {
            lua.create_userdata(LuaDateTime {
                inner: DateTime::now(),
            })
        })?,
    )?;

    Ok(table)
}

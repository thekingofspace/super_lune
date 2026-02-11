use std::{
    io::Error,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use async_lock::Mutex as AsyncMutex;
use async_net::TcpListener;
use bstr::BString;
use futures::{
    io::{ReadHalf, WriteHalf},
    prelude::*,
};
use mlua::prelude::*;

use crate::client::stream::MaybeTlsStream;

const DEFAULT_BUFFER_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct Tcp {
    local_addr: Arc<Option<SocketAddr>>,
    remote_addr: Arc<Option<SocketAddr>>,
    read_half: Arc<AsyncMutex<ReadHalf<MaybeTlsStream>>>,
    write_half: Arc<AsyncMutex<WriteHalf<MaybeTlsStream>>>,
}

impl Tcp {
    async fn read(&self, size: usize) -> Result<Option<Vec<u8>>, Error> {
        let mut buf = vec![0; size];

        loop {
            let mut handle = self.read_half.lock().await;
            let read = handle.read(&mut buf).await?;

            if read == 0 {
                return Ok(None);
            }

            if read > 0 {
                buf.truncate(read);
                return Ok(Some(buf));
            }
        }
    }

    async fn write(&self, data: Vec<u8>) -> Result<(), Error> {
        let mut handle = self.write_half.lock().await;
        handle.write_all(&data).await?;
        Ok(())
    }

    async fn close(&self) -> Result<(), Error> {
        let mut handle = self.write_half.lock().await;
        handle.close().await?;
        Ok(())
    }

    fn host_type(&self) -> String {
        let Some(remote) = self.remote_addr.as_ref() else {
            return "unknown".to_string();
        };

        match remote.ip() {
            IpAddr::V4(v4) => {
                if v4.is_loopback() {
                    "localhost"
                } else if v4.is_private() {
                    "lan"
                } else {
                    "internet"
                }
            }
            IpAddr::V6(v6) => {
                if v6.is_loopback() {
                    "localhost"
                } else if v6.is_unique_local() {
                    "lan"
                } else {
                    "internet"
                }
            }
        }
        .to_string()
    }
}

impl<T> From<T> for Tcp
where
    T: Into<MaybeTlsStream>,
{
    fn from(value: T) -> Self {
        let stream = value.into();

        let local_addr = stream.local_addr().ok();
        let remote_addr = stream.remote_addr().ok();
        let (read, write) = stream.split();

        Self {
            local_addr: Arc::new(local_addr),
            remote_addr: Arc::new(remote_addr),
            read_half: Arc::new(AsyncMutex::new(read)),
            write_half: Arc::new(AsyncMutex::new(write)),
        }
    }
}

impl LuaUserData for Tcp {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("localIp", |_, this| {
            Ok(this.local_addr.as_ref().map(|a| a.ip().to_string()))
        });

        fields.add_field_method_get("localPort", |_, this| {
            Ok(this.local_addr.as_ref().map(|a| a.port()))
        });

        fields.add_field_method_get("remoteIp", |_, this| {
            Ok(this.remote_addr.as_ref().map(|a| a.ip().to_string()))
        });

        fields.add_field_method_get("remotePort", |_, this| {
            Ok(this.remote_addr.as_ref().map(|a| a.port()))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("read", |lua, this, size: Option<usize>| {
            let this = this.clone();
            let size = size.unwrap_or(DEFAULT_BUFFER_SIZE);

            async move {
                match this.read(size).await.into_lua_err()? {
                    Some(bytes) => Ok(LuaValue::String(lua.create_string(bytes)?)),
                    None => Ok(LuaValue::Nil),
                }
            }
        });

        methods.add_async_method("write", |_, this, data: BString| {
            let this = this.clone();
            let data = data.to_vec();
            async move { this.write(data).await.into_lua_err() }
        });

        methods.add_async_method("close", |_, this, (): ()| {
            let this = this.clone();
            async move { this.close().await.into_lua_err() }
        });

        methods.add_method("host", |_, this, ()| Ok(this.host_type()));
    }
}

#[derive(Clone)]
pub struct TcpHost {
    listener: Arc<TcpListener>,
    local_addr: SocketAddr,
}

impl TcpHost {
    pub async fn new(addr: String, port: u16) -> Result<Self, Error> {
        let bind_addr = format!("{addr}:{port}");
        let listener = TcpListener::bind(&bind_addr).await?;
        let local_addr = listener.local_addr()?;
        Ok(Self {
            listener: Arc::new(listener),
            local_addr,
        })
    }

    async fn accept(&self) -> Result<Tcp, Error> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Tcp::from(stream))
    }

    fn close(&self) -> Result<(), Error> {
        drop(self.listener.clone());
        Ok(())
    }
}

impl LuaUserData for TcpHost {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("localIp", |_, this| Ok(this.local_addr.ip().to_string()));

        fields.add_field_method_get("localPort", |_, this| Ok(this.local_addr.port()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("accept", |_, this, (): ()| {
            let this = this.clone();
            async move {
                let client = this.accept().await.into_lua_err()?;
                Ok(client)
            }
        });

        methods.add_method("close", |_, this, (): ()| this.close().into_lua_err());
    }
}

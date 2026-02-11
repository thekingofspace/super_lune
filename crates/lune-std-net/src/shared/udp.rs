use std::sync::Arc;

use async_net::UdpSocket;
use mlua::prelude::*;

#[derive(Clone)]
pub struct Udp {
    socket: Arc<UdpSocket>,
}

impl Udp {
    pub async fn bind(port: u16) -> LuaResult<Self> {
        let addr = format!("0.0.0.0:{port}");

        let socket = UdpSocket::bind(addr).await.map_err(LuaError::external)?;

        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub async fn connect(host: String, port: u16) -> LuaResult<Self> {
        let addr = format!("{host}:{port}");

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(LuaError::external)?;

        socket.connect(addr).await.map_err(LuaError::external)?;

        Ok(Self {
            socket: Arc::new(socket),
        })
    }
}

impl LuaUserData for Udp {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("send", |_, this, data: LuaString| async move {
            let bytes = data.as_bytes();
            this.socket.send(&bytes).await.map_err(LuaError::external)?;
            Ok(())
        });

        methods.add_async_method(
            "sendTo",
            |_, this, (data, host, port): (LuaString, String, u16)| async move {
                let addr = format!("{host}:{port}");
                let bytes = data.as_bytes();

                this.socket
                    .send_to(&bytes, addr)
                    .await
                    .map_err(LuaError::external)?;
                Ok(())
            },
        );

        methods.add_async_method("recv", |lua, this, ()| async move {
            let mut buf = vec![0u8; 65535];

            let (len, addr) = this
                .socket
                .recv_from(&mut buf)
                .await
                .map_err(LuaError::external)?;

            let data = lua.create_string(&buf[..len])?;

            Ok((data, addr.ip().to_string(), addr.port()))
        });

        methods.add_method("localAddr", |_, this, ()| {
            let addr = this.socket.local_addr().map_err(LuaError::external)?;
            Ok((addr.ip().to_string(), addr.port()))
        });

        methods.add_method("close", |_, _this, ()| Ok(()));
    }
}

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use anyhow::{anyhow, bail};
use axum::body::Bytes;
use tower::{MakeService, Service, service_fn};
use wasi_common::pipe::{ReadPipe, WritePipe};

use wasmtime::{Config, Engine, Module, Store, Linker, Instance, TypedFunc};

use wasmtime_wasi::{
    WasiCtx,
    tokio::{
        add_to_linker,
        WasiCtxBuilder,
    },
};


const PAYLOAD_SIMPLEST: &[u8] = include_bytes!("../simplest.wasm");

#[derive(Clone)]
pub struct Wasm {
    engine: Engine,
    linker: Linker<WasiCtx>,
    modules: HashMap<String, Module>,
}

impl Wasm {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();

        config.async_support(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;

        let mut linker = Linker::new(&engine);

        add_to_linker(&mut linker, |cx| cx)?;

        // load wasm modules
        let simplest = Module::from_binary(&engine, PAYLOAD_SIMPLEST)?;

        let modules = HashMap::from_iter([
            ("simplest".into(), simplest)
        ]);

        Ok(Wasm {
            engine,
            linker,
            modules,
        })
    }

    pub async fn run(&self, module: String, payload: Bytes) -> anyhow::Result<Vec<u8>> {
        let module = self.modules.get(&module).
            ok_or(anyhow!("module not found: {}", module))?;

        let stdin = ReadPipe::from(payload.as_ref());
        let stdout = WritePipe::new_in_memory();

        let wasi = WasiCtxBuilder::new()
            .stdout(Box::new(stdout.clone()))
            .stdin(Box::new(stdin))
            .build();

        let mut store = Store::new(&self.engine, wasi);

        store.out_of_fuel_async_yield(u64::MAX, 10000);

        let instance = self.linker
            .instantiate_async(&mut store, &module)
            .await?;

        instance
            .get_typed_func::<(), (), _>(&mut store, "_start")?
            .call_async(&mut store, ())
            .await?;

        // look at WritePipe.try_into_inner
        drop(store);

        let output: Vec<u8> = stdout
            .try_into_inner()
            .expect("")// TODO: replace with map_err
            .into_inner();

        Ok(output)
    }
}


#[derive(Clone)]
pub struct WasmSvc {
    inner: Wasm,
}

impl WasmSvc {
    pub fn new(inner: Wasm) -> Self {
        Self {
            inner,
        }
    }
}

impl Service<(String, Bytes)> for WasmSvc {
    type Response = Vec<u8>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, (module, payload): (String, Bytes)) -> Self::Future {
        let inner = self.inner.clone();

        Box::pin(async move {
            inner.run(module, payload).await
        })
    }
}

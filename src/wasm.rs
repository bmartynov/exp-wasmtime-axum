use std::collections::HashMap;

use anyhow::anyhow;

use wasmtime::{Config, Engine, Module, Store, Linker};

use wasi_common::WasiFile;

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

    pub async fn run(&self, module: String, stdin: Box<dyn WasiFile>, stdout: Box<dyn WasiFile>) -> anyhow::Result<()> {
        let module = self.modules.get(&module)
            .ok_or(anyhow!("module not found: {}", module))?;

        let wasi = WasiCtxBuilder::new()
            .stdout(stdout)
            .stdin(stdin)
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

        Ok(())
    }
}

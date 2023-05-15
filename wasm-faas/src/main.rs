use std::sync::{Arc, RwLock};
use std::io;
use std::result;
use std::collections::HashMap;

use wasmtime::{Engine, Linker, Module as Module_time, Store};
use wasmtime_wasi::sync::WasiCtxBuilder;
use wasi_common::pipe::{WritePipe};

use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use actix_web::web::{Query, Path};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use wasmedge_sdk::{
    config::{CommonConfigOptions, ConfigBuilder, HostRegistrationConfigOptions},
    params,
    plugin::PluginManager,
    Module, VmBuilder,
};

/// run models by `wasmtime` runtime
///
/// * `module_name`: 
/// * `params`: 
fn invoke_wasmtime_module(module_name: String, params: HashMap<String, String>)
    -> result::Result<String, wasmtime_wasi::Error> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    let stdout_buf: Vec<u8> = vec![];
    let stdout_mutex = Arc::new(RwLock::new(stdout_buf));
    let stdout = WritePipe::from_shared(stdout_mutex.clone());

    // convert params hashmap to an array
    let envs: Vec<(String,String)>  = params.iter().map(|(key, value)| {
        (key.clone(), value.clone())
    }).collect();

    let wasi = WasiCtxBuilder::new()
        .stdout(Box::new(stdout))
        .envs(&envs)?
        .build();
    let mut store = Store::new(&engine, wasi);
    
    let module = Module_time::from_file(&engine, &module_name)?;
    linker.module(&mut store, &module_name, &module)?;

    let instance = linker.instantiate(&mut store, &module)?;
    let instance_main = instance.get_typed_func::<(), (), _>(&mut store, "_start")?;
    instance_main.call(&mut store, ())?;

    let mut buffer: Vec<u8> = Vec::new();
    stdout_mutex.read().unwrap().iter().for_each(|i| {
        buffer.push(*i)
    });

    let s = String::from_utf8(buffer)?;
    Ok(s)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn infer() -> Result<(), Box<dyn std::error::Error>> {
    use std::str::FromStr;

    // parse arguments
    // let args: Vec<String> = std::env::args().collect();
    // let dir_mapping = &args[1];
    // let wasm_file = &args[2];
    // let model_bin = &args[3];
    // let image_file = &args[4];
    
    let dir_mapping = ".:.";
    let wasm_file = "wasmedge-wasinn-example-mobilenet-image.wasm";
    let model_bin = "mobilenet.pt";
    let image_file = "input.jpg";

    dbg!("load plugin");

    // load wasinn-pytorch-plugin
    // TODO: Change to a base path
    let p = std::path::PathBuf::from_str("/home/wasm/.wasmedge/plugin")?;
    PluginManager::load(Some(p.as_path()))?;

    let config = ConfigBuilder::new(CommonConfigOptions::default())
        .with_host_registration_config(HostRegistrationConfigOptions::default().wasi(true))
        .build()?;
    assert!(config.wasi_enabled());
    // assert!(config.wasi_nn_enabled());

    // load wasm module from file
    let module = Module::from_file(Some(&config), wasm_file)?;

    // create a Vm
    let mut vm = VmBuilder::new()
        .with_config(config)
        .with_plugin_wasi_nn()
        .build()?
        .register_module(Some("extern"), module)?;

    // init wasi module
    vm.wasi_module_mut()
        // .ok_or("Not found wasi module")?
        .expect("Not found wasi module")
        .initialize(
            Some(vec![wasm_file, model_bin, image_file]),
            None,
            Some(vec![dir_mapping]),
        );

    vm.run_func(Some("extern"), "_start", params!())?;
    Ok(())
}

#[get("/wasmtime/{module_name}")]
async fn handler_wasmtime(module_name: Path<String>, query: Query<HashMap<String, String>>)
    -> impl Responder {
    let wasm_module = format!("{}{}", module_name, ".wasm");  
    let val = invoke_wasmtime_module(wasm_module, query.into_inner()).expect("invocation error");
    HttpResponse::Ok().body(val)
}

#[get("/wasmedge/infer")]
async fn handler_wasmedge(query: Query<HashMap<String, String>>)
    -> impl Responder {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    infer().expect("wasmedge handler error");
    
    HttpResponse::Ok()
}


#[actix_web::main]
async fn main() -> io::Result<()> {
    println!("Listen at 127.0.0.1:8080");
    HttpServer::new(|| {
            App::new()
            .service(handler_wasmedge)
            .service(handler_wasmtime)
        })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

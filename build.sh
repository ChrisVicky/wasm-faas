#!/bin/sh

mkdir target
rm ./target/*

# build wasm-faas with wasmtime
(cd wasm-faas && cargo build --release)
cp wasm-faas/target/debug/wasm-faas ./target

# build examples
(cd examples/hello-world-as && npm run build)
cp examples/hello-world-as/build/hello-world.wasm ./target

(cd examples/option-pricing-as && npm run build)
cp examples/option-pricing-as/build/option-pricing.wasm ./target

# Build wasm32-wasi
(cd examples/sudoku-rs && cargo build --release --target wasm32-wasi)
cp examples/sudoku-rs/target/wasm32-wasi/release/sudoku-rs.wasm ./target

(cd examples/hello-rust && cargo build --release --target wasm32-wasi)
cp examples/hello-rust/target/wasm32-wasi/release/hello-rust.wasm ./target

# Build wasm32-wasi -> pytorch example
(cd ./examples/pytorch-mobilenet-image/rust/ && cargo build --release --target wasm32-wasi)
cp ./examples/pytorch-mobilenet-image/rust/target/wasm32-wasi/release/wasmedge-wasinn-example-mobilenet-image.wasm ./target
cp ./examples/pytorch-mobilenet-image/mobilenet.pt ./target 
cp ./examples/pytorch-mobilenet-image/input.jpg ./target/

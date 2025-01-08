#!/bin/bash

# wasm-pack build --debug --target web
# wasm-pack build --target nodejs
wasm-pack build --target web && cp ./pkg/zero_sugar.js ./src/web && cp ./pkg/zero_sugar_bg.wasm ./src/web

echo "

";

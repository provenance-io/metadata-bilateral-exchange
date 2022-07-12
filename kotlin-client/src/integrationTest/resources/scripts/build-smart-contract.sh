#!/bin/sh
wasm_output_directory=./../smart-contract/artifacts/
wasm_directory=./src/integrationTest/resources/artifacts
wasm_file="$wasm_directory/metadata_bilateral_exchange.wasm"
if test -f "$wasm_file"; then
  echo "Removing old WASM file"
  rm "$wasm_file"
  echo "Successfully removed old wasm file.  Regenerating it now..."
else
  echo "Generating WASM file for testing..."
fi
(cd ./../smart-contract && make optimize)
if test -f "$wasm_directory"; then
  echo "WASM project output directory already exists. No need to create it"
else
  echo "Creating WASM output directory: $wasm_directory"
  mkdir "$wasm_directory"
fi
echo "Copying WASM file to output directory: $wasm_directory"
cp "$wasm_output_directory/metadata_bilateral_exchange.wasm" "$wasm_file"


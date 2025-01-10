## Solitaire - Rust


#### Reason behind this project
This is the project for the ATAD course from UPT - IT Master - 2nd Year.

#### What is this?
This a remake of a Solitaire game I have created in python and wanted to see if I can remake it using Rust and WASM.

#### How to run?
Go to the `solitaire-wasm` directory. There, you can run the following commands in order to build the WASM part and to access the game:

```bash
# Build the WASM part
wasm-pack build --target web

# In order to access the game, run from the src/static
python -m http.server
```

Afterwards, you can just access the game on `localhost:8000`.

#### Versions used

```bash
rustc --version
rustc 1.83.0

cargo --version
cargo 1.83.0

wasm-pack --version
wasm-pack 0.13.1
```
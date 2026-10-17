# 1. Instalar dependências e o rustup
pkg install curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 2. Carregar o ambiente do cargo/rustup
source $HOME/.cargo/env

# 3. Adicionar o target WebAssembly
rustup target add wasm32-unknown-unknown

# 4. Compilar o projeto
cargo build --release --target wasm32-unknown-unknown

# 5. Servir para testar no navegador do celular
python -m http.server 8000

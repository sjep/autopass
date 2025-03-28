SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
HOME_BIN="$HOME/bin"
mkdir -p "${HOME_BIN}"
cd "${SCRIPT_DIR}"

cargo build --release --bin=ap --features=gui
cargo build --release --bin=apcli --features=cli

cp target/release/ap "${HOME_BIN}"
cp target/release/apcli "${HOME_BIN}"

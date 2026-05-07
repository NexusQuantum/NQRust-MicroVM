# nqvm CLI

`nqvm` is the command-line client for the NQR-MicroVM manager API.

## Build

```bash
cargo build -p nqvm-cli
```

## Examples

```bash
cargo run -p nqvm-cli -- login --api-url http://localhost:18080 --username root
nqvm vm list
nqvm vm create --name dev --vcpu 2 --mem-mib 2048 --rootfs-image-id <uuid> --kernel-image-id <uuid>
nqvm vm create --file vm.yaml
nqvm vm shell <vm-id>
nqvm container deploy --file container.yaml
nqvm function invoke <id> --file event.json --output json
```

The default config path is `~/.config/nqvm/config.toml`:

```toml
api_url = "http://127.0.0.1:18080"
token = "..."
default_output = "table"
```

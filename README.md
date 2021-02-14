# GameRoom - PoC

Proof of Concept of Game Room in Actix Websocket

## Command Line Help

- Bash Shell:

```bash
> RUSTFLAGS="-C target-cpu=native -C link-args=-s" cargo run --release -- --help
```

- PowerShell:

```powershell
> $env:RUSTFLAGS="-C link-args=-s -C target-feature=+crt-static -C target-cpu=native"
> cargo run --release -- --help
```

- Result:

```text
game-room 0.1.0
PoC - Game Room Router

USAGE:
    game-room [FLAGS] [OPTIONS]

FLAGS:
    -d, --debug-mode    
    -h, --help          Prints help information
    -V, --version       Prints version information

OPTIONS:
    -l, --listen-port <listen-port>    Set listening port [default: 7575]
    -s, --server-uuid <server-uuid>    Set server UUID/GUID [default: 00000000-0000-0000-0000-000000000000]
```

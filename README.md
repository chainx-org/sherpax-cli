# sherpax-cli

## Check Balance

```bash
$ ./target/release/sherpax-cli check-balance --help
sherpax-cli-check-balance 0.2.0
Arguments required for creating and sending an extrinsic to a sherpax node

USAGE:
    sherpax-cli check-balance [FLAGS] [OPTIONS]

FLAGS:
    -h, --help             Prints help information
        --print-details    Enable print the detail info(every 10000 blocks)
    -V, --version          Prints version information

OPTIONS:
        --block-number <block-number>    The specified block number
        --url <url>                      Websockets url of a sherpax node [default: ws://localhost:9977]

```

```bash
$ ./target/release/sherpax-cli check-balance --url ws://127.0.0.1:9977
{
  "block_number":1311526,
  "locked":"11052331985778826625796863",
  "reserved":"1005836000000000000000",
  "transferable_exclude_treasury":"1642896563029881356280018",
  "treasury_balance":"8303769065775200000000000"
}
```

## Generate metadata

```bash
cargo install subxt-cli

subxt metadata -f bytes --url http://localhost:8546 > sherpax_metadata.scale
```

gh-device-flow
==============

Crate that implements the GitHub Device Flow for authenticating with GitHub Apps.

It also implements the refresh mechanism for GitHub Apps that use GitHub's token
expiration feature.


## Usage

To use the CLI, download the binary for your architecture/OS from [the latest release](https://github.com/jakewilkins/gh-device-flow/releases/latest).

Once extracted, execute the command providing the Client ID for your App:

```bash
github-device-flow --client-id Iv1.8675309ABCDEFGH
```

This will prompt you to open a browser window and provide the generated device code. Once completed, your access token will be printed to STDOUT as a JSON object. If your App requests refresh tokens one will also be printed.

To refresh your OAuth Access using a Refresh Token, pass it as a `--refresh` flag:

```bash
github-device-flow --client-id Iv1.8675309ABCDEFGH --refresh thisisnotarefreshtoken
```

To view the full help, pass the `--help` flag:

```
$ github-device-flow --help
github-device-flow 0.1.2
Binary and library for performing the GitHub Device Flow

USAGE:
    github-device-flow [OPTIONS] --client-id <CLIENT_ID>

OPTIONS:
    -c, --client-id <CLIENT_ID>    Client ID
    -h, --host <HOST>              The host to authenticate with
        --help                     Print help information
    -r, --refresh <REFRESH>        A Refresh Token to exchange
    -s, --scope <SCOPE>            The scope required for the auth app
    -V, --version                  Print version information
```

## Library usage

By default the crate uses `reqwest`'s blocking client:

```rust
let cred = github_device_flow::authorize("Iv1.8675309ABCDEFGH".into(), None, None)?;
```

For async code, enable the `async` feature (requires a Tokio runtime). Add
`default-features = false` if you don't also want the blocking API:

```toml
github-device-flow = { version = "0.2", default-features = false, features = ["async"] }
```

```rust
let cred = github_device_flow::nonblocking::authorize("Iv1.8675309ABCDEFGH".into(), None, None).await?;
```

`DeviceFlow` also has `start_async`, `setup_async`, `update_async` and `poll_async`
alongside the blocking `start`, `setup`, `update` and `poll`. See `examples/async.rs`.

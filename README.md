# Tracer
HTTP request utility to collect timing statistics about requests to endpoints.

## Usage
### Tracer
`tracer help`:

```
USAGE:
    tracer [FLAGS] [OPTIONS] <URL>
    tracer [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -C, --continuous     Continuous mode
        --capture-all    Capture all headers from response
    -h, --help           Prints help information
    -s, --stats          Show statistics at completion
    -v                   Sets verbosity level
    -V, --version        Prints version information

OPTIONS:
    -f, --body <BODY_FILE>       File to use as request body
    -i, --capture <HEADER>...    Header to capture from request. Can be specified multiple times. Case insensitive.
    -H, --header <HEADER>...     Header to include in request, in HEADER=VALUE format.  Can be specified multiple times.
                                 Case insensitive
    -X, --method <METHOD>        HTTP Method to use (Default GET)
    -n <COUNT>                   Repeat request a set number of times

ARGS:
    <URL>    URL to test

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    test    Run pre-defined tests in toml format
```

### Tracer Test
`tracer help test`:

```
USAGE:
    tracer test <CONFIG>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <CONFIG>    Config file that specifies the test(s) to run
```

**Note**: the `-C`, `-n <COUNT>`, and `-s` flags can be used with `tracer test` to repeat tests. Invoke like `tracer -C test ...`, `tracer -n 5 test ...`, or `tracer -s test ...`

### Pre-defined Tests
Tests can be defined in TOML format to enhance repeatability and then invoke it with `tracer test path/to/test.toml`.

```toml
#Optional section, and all keys are optional as well. Values will be used as fallbacks if tests do not define them.
[defaults]
# URL to connect to. If there is no default url and no test url, an error will be returned
url = "https://www.google.com"
# HTTP Method to use. Default is GET even if `method` is never specified
method = "GET"

# Subsection of defaults to specify Response Headers that should be printed out. Only one of `all` or `list` can be specified
[defaults.capture_headers]
# Capture all headers
all = true
# Capture a specific list of headers (case insensitive)
list = [ "Cache-Control" ]

# Subsection of defaults to specify Request Headers that should be included in the request
[defaults.headers]
Accept = "*/*"

# [[test]] sections are repeatable and define the tests to run
[[test]]
# required
name = "My Test"
# Optional, falls back to default
url = "http://localhost"
# Optional, falls back to default
method = "POST"
# Optional, specify request body.  Can be either `file`, which is a path relative to the test .toml that contains the body content, or `value` with the body specified inline
payload = { file = "data.json" }

# Optional, defaults to `defaults.headers`
[test.headers]
Content-Type = "application/json"

# Optional, defaults to `defaults.capture_headers`
[test.capture_headers]
all = true
```

## Examples

```
$ tracer -n5 -s https://www.google.com
* https://www.google.com/ (200 OK) Hash: e08cff4f DNS: 39ms Conn: 25ms TLS: 66ms Hdrs: 207ms Resp: 253ms 
* https://www.google.com/ (200 OK) Hash: 1b11b899 DNS: 0ms Conn: 24ms TLS: 64ms Hdrs: 160ms Resp: 203ms 
* https://www.google.com/ (200 OK) Hash: 6a48961a DNS: 1ms Conn: 27ms TLS: 63ms Hdrs: 164ms Resp: 209ms 
* https://www.google.com/ (200 OK) Hash: 4763a261 DNS: 0ms Conn: 25ms TLS: 63ms Hdrs: 171ms Resp: 214ms 
* https://www.google.com/ (200 OK) Hash: 724ba633 DNS: 0ms Conn: 25ms TLS: 60ms Hdrs: 164ms Resp: 208ms 
https://www.google.com/ stats:
  Dns: count 5/min 0ms/avg 8ms/max 39ms/stdev 15ms
  Connection: count 5/min 24ms/avg 25ms/max 27ms/stdev 0ms
  Tls: count 5/min 60ms/avg 63ms/max 66ms/stdev 1ms
  Headers: count 5/min 160ms/avg 173ms/max 207ms/stdev 17ms
  FullResponse: count 5/min 203ms/avg 218ms/max 253ms/stdev 17ms
```

### Explanation
* Hash - SHA256 hash of response body, abbreviated to first 8 hex digits
* DNS - time taken to resolve DNS name (Omitted if connecting to an IP)
* Conn/Connection - time taken to establish TCP connection
* TLS - time taken to do TLS negotiation (Omitted if connecting over plain HTTP)
* Hdrs/Headers - time taken to receive the HTTP headers, starting from initiation of the request
* Resp/FullResponse - time taken to receive the full response body, starting from initiation of the request

## Building
Tracer uses the 2018 edition of Rust and therefore depends on a rust version >= 1.31.0.

### Standard
#### Debug Builds
`cargo build`. Binary will be `target/debug/tracer`.

#### Release Builds
`cargo build --release`. Binary will be `target/release/tracer`.

### MUSL libc (fully static builds)
MUSL target will need to be installed. If using `rustup`, run `rustup target add <arch>-unknown-linux-musl`.
The rest of this section assumes `<arch>=x86_64`. For a full list of possible targets, see [the Rust Forge](https://forge.rust-lang.org/platform-support.html)

#### Debug Builds
`cargo build --target x86_64-unknown-linux-musl`.  Binary will be `target/x86_64-unknown-linux-musl/debug/tracer`
#### Release Builds
`cargo build --release --target x86_64-unknown-linux-musl`. Binary will be `target/x86_64-unknown-linux-musl/release/tracer`

### This binary is huge!
Yep. You can cut it roughly in half by running `strip(1)`: `strip /path/to/tracer`. 

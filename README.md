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

## Building

### GNU libc
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

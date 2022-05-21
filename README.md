# PiRESENCE - PIR-based presence detection

A word of warning, first: this is an ongoing, unfinished project I'm developing for fun, mainly as an excuse to learn Rust.

This is the source repository of PiRESENCE, a (wannabe) Home Assistant (HA) addon for room presence detection.

The addon (will) listen to events from motion detector sensor via HA, update the current status
and predict the presence state for each interconnected room, providing the informaton back to
HA to be used for triggers or conditions.

## Cross-Compilation for Raspberry 4

These sections below show how to cross-compile this project for the `aarch64-linux-unknown-musl` target.

### On Ubuntu

Download and extract somewhere the aarch64 cross compilation tools for MUSL:

```bash
wget https://musl.cc/aarch64-linux-musl-cross.tgz
tar xvzf aarch64-linux-musl-cross.tgz -C /path/to/destination/
```

Append the path to the extracted toolchain to your PATH environment variable, e.g.
on `~/.bashrc`:

```bash
export PATH="$PATH:/path/to/destination/bin"
```

Reload your shell, then make sure the cargo config file has a section like the following one:

```toml
[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-ld"
```

Finally, add the target with rustup:

```bash
rustup add target aarch64-unknown-linux-musl
```

From now on, the project may be compiled for Alpine aarch64 with the cargo command:

```bash
cargo build --target=aarch64-unknown-linux-musl
```


### With cross

Make sure to have Docker installed. Then, install `cross`:

```bash
cargo install cross
```

You may now compile the target like this:

```bash
cross build --release --target=aarch64-unknown-linux-musl
```

## License

PiRESENCE is distributed under the terms of both the MIT license and
the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for
details.

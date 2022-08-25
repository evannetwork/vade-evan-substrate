# Vade Evan Substrate Plugin

[![crates.io](https://img.shields.io/crates/v/vade-evan-substrate.svg)](https://crates.io/crates/vade-evan-substrate)
[![Documentation](https://docs.rs/vade-evan-substrate/badge.svg)](https://docs.rs/vade-evan-substrate:q)
[![Apache-2 licensed](https://img.shields.io/crates/l/vade-evan-substrate.svg)](./LICENSE.txt)

## About

This crate allows you to use to work with DIDs Trust and Trace, which runs on evan.network.
For this purpose a [`VadePlugin`] implementation is exported: [`VadeEvanSubstrate`].

## VadeEvanSubstrate

Supports creating, updating and getting DIDs and DID documents on substrate, therefore supports:

- [`did_create`]
- [`did_resolve`]
- [`did_update`]

### Signing substrate requests

As the did resolver instance needs to sign its requests against substrate, a remote endpoint for signing has to be provided. The DID resolver will sign requests for [`did_create`] and [`did_update`]. A signing endpoint has to be passed with the config argument in the constructor, e.g.:

```rust
use vade_evan_substrate::{ResolverConfig, VadeEvanSubstrate};
use vade_signer::{LocalSigner, Signer},
let signer: Box<dyn Signer> = Box::new(LocalSigner::new());
let resolver = VadeEvanSubstrate::new(ResolverConfig {
    signer,
    target: "127.0.0.1".to_string(),
});
```

When signing remotely, `signing_url` will be called with a POST request. The messages that should be signed is passed to the server alongside a reference to a key like this:

```json
{
    "key": "some-key-id",
    "type": "some-key-type",
    "message": "sign me please"
}
```

Two types of of responses are supported. Successful signing results are give in this format:

```json
{
  "messageHash": "0x52091d1299031b18c1099620a1786363855d9fcd91a7686c866ad64f83de13ff",
  "signature": "0xc465a9499b75eed6fc4f658d1764168d63578b05ae15305ceedc94872bda793f74cb850c0683287b245b4da523851fbbe37738116635ebdb08e80b867c0b4aea1b",
  "signerAddress": "0x3daa2c354dba8d51fdabc30cf9219b251c74eb56"
}
```

Errors can be signaled this way:

```json
{
    "error": "key not found"
}
```

## Compiling vade-evan-substrate

### "Regular" build

No surprise here:

```sh
cargo build --release
```

### WASM

To compile `vade-evan-substrate` for wasm, use wasm pack.

Also you have to specify whether to build a browser or a nodejs environment.

nodejs:

```sh
wasm-pack build --release --target nodejs
```

browser:

```sh
wasm-pack build --release --target web
```

[`did_create`]: https://docs.rs/vade_evan_substrate/*/vade_evan_substrate/resolver/struct.VadeEvanSubstrate.html#method.did_create
[`did_resolve`]: https://docs.rs/vade_evan_substrate/*/vade_evan_substrate/resolver/struct.VadeEvanSubstrate.html#method.did_resolve
[`did_update`]: https://docs.rs/vade_evan_substrate/*/vade_evan_substrate/resolver/struct.VadeEvanSubstrate.html#method.did_update
[`VadeEvanSubstrate`]: https://docs.rs/vade_evan_substrate/*/vade_evan_substrate/resolver/struct.VadeEvanSubstrate.html
[`Vade`]: https://docs.rs/vade/*/vade/struct.Vade.html
[`VadePlugin`]: https://docs.rs/vade/*/vade/trait.VadePlugin.html

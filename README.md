# Linux uapi Headers for Rust (currently failing)

This project attempts to generate a Rust library that mirrors all the Linux kernel uapi headers.

Currently it will generate ~77% of them successfully, 616 out of 804.
This results in 19MB of generated code, or ~382,000 lines according to [`tokei`](https://github.com/Aaronepower/tokei):

```
$ tokei src
-------------------------------------------------------------------------------
 Language            Files        Lines         Code     Comments       Blanks
-------------------------------------------------------------------------------
 Rust                  805       384013       381654          578         1781
-------------------------------------------------------------------------------
```

## Why it fails to build

Currently bindgen generates some invalid code and if you attempt to compile this project you will get the following error:

```
$ cargo build
   Compiling linux-uapi v0.4.9 (file:///home/ferris/dev/linux-uapi.rs)
error: expected identifier, found `_`
   --> src/linux/atm_tcp.rs:180:9
    |
180 |     pub _: [::std::os::raw::c_uchar; 8usize],
    |         ^
    |
    = note: `_` is a wildcard pattern, not an identifier

error: aborting due to previous error

error: Could not compile `linux-uapi`.

To learn more, run the command again with --verbose.
```

There's already an issue for this in the bindgen repository ([#631](https://github.com/rust-lang-nursery/rust-bindgen/issues/631)) and I've added a message about this to the issue.

## Why it fails to generate 23% of the files

I don't know.
My immediate goal with this was to get a partial build working but this has been stalled by the error explained above.
Until that's fixed I won't look towards fixing other generation issues.
To see which bindings currently fail checkout `failed_bindings.txt` in the root of this repository.

## How to build

Clone whatever version of the Linux kernel you want to use, for this example I'll use version 4.9.

```bash
$ git clone https://github.com/torvalds/linux.git --branch v4.9 --depth 1
```

Then run `cargo build` which will kick off the `build.rs` script and run for ~3 minutes generating all the bindings before attempting (and currently failing) to compile.

## Debugging build

Because printing to stdout inside of `build.rs` is simply passed to cargo as command line arguments, `build.rs` creates a fresh `build_rs.log` file during each run and writes debug information to it. If you have issues scan back through it at least get a rough idea of what happened.

## Project vision

This projects goal is to be able to generate bindings to 100% of the Linux uapi, thus allowing rust projects to not have to constantly reimplement all the structs and enums which aren't popular enough to be included in `libc`.

## When using NixOS

You'll need to manually export `LIBCLANG_PATH` to the appropriate place (yes this is hacky for now).
Example: `export LIBCLANG_PATH="/nix/store/k890b7cj7zz6i06xijpgd3d05nxicvp3-clang-3.8.1/lib/"`

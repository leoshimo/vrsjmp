# vrs

[![Rust](https://github.com/leoshimo/vrs/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/leoshimo/vrs/actions/workflows/rust.yml)

🚧 Under Heavy Construction

A personal software runtime.

> In the multiverse, you can live up to your ultimate potential. We discovered a
> way to temporarily link your consciousness to another version of yourself,
> accessing all of their memories and skills.
>
> It's called verse jumping.
>
> — Alpha Waymond

[Renaissance tech for renaissance people](https://web.archive.org/web/20210428062809/https://twitter.com/dhh/status/1341758748717510659)

## Structure

- `libvrs`: The `vrs` library crate shared by runtime and clients
- `vrsd`: A daemon runtime implementation
- `vrsctl`: A CLI client for interacting with runtime
- `lemma`: Embedded Lisp that is substrate for code and data

## What is this?

This is a WIP personal software runtime so I can write software the way I want to.

My inspirations are:

- Emacs
- Unix
- Plan 9
- Hypermedia systems
- Erlang

and the goal is to bundle my favorite parts from each of those systems into one software runtime.

## Debugging

Use `RUST_LOG` to configure logging level:

```sh
$ RUST_LOG=debug cargo run --bin vrsd
```

### Thoughts / Quotes that map project in latent space


> The experience of `emacs` everywhere

> If `xdg-open`, `systemd`, and `dbus` raised a child in an alternative universe where Lisp replaced XML as dominant configuration language

> The thing about ideas, is that ideas start out small... tiny and weak and fragile... Ideas need an environment where creator can nurture them.. feed them and shape their growth

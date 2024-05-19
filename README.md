# 🦀🧠 rust_brain

An implementation of a [brainfuck](https://en.wikipedia.org/wiki/Brainfuck) interpreter in Rust accompanied by a X86_64 JIT for it.

**WARNING:** The interpreter was an explanation vehicle for a [youtube video series](https://www.youtube.com/playlist?list=PLy68GuC77sURmAfuSedQYRxgG9ORG6MnP), as well as a means to explore how an easy JIT can be written. It is therefore not a fully fledged optimal brainfuck implementation.

## Building

The project utilizes the Cargo build system. To build the project, run the following command in the terminal:

```shell
cargo build --release
```

## Execution

To execute the interpreter/jit, run it from the command line with the brainfuck source code to interpret as the first argument. For example:

```shell
target/release/rust_brain examples/hello_world.brainfuck
```

On a compatible system (X86_64/linux) the jit will automatically be chosen, otherwise the interpreter will be spun up.

## Purpose

This interpreter was developed purely for the enjoyment of coding. There is no practical use case for the brainfuck language or this interpreter. However, if you wish to join in the fun and follow the development process, there are videos on my [YouTube channel](https://www.youtube.com/@MrJakob) showcasing the different stages of its creation.

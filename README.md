# EraVM

## Requirements

- [Rust](https://www.rust-lang.org/tools/install)
- [The EraVM Compiler: zksolc 1.5.0](https://github.com/matter-labs/zksolc-bin). Download the [latest binary](https://github.com/matter-labs/zksolc-bin/releases/tag/v1.5.0, then put it under your path. If done correctly, running `zksolc --version` should return 1.5.0.
- [Cargo nextest](https://nexte.st/#cargo-nextest)

## Compiling programs

Note: This is a bit rough right now, it'll get better in the coming days when we add proper testing and other stuff.

To try out a program, write it in yul and put it under the `programs` directory, then run

```
make compile-programs
```

which compiles every program and puts the resulting artifacts in the `program_artifacts` directory.

As an example, there's an `add.yul` sample program right now. After compiling it, you'll see (among others) the following files in `program_artifacts/add.artifacts`:

- `add.yul.zasm`: The resulting `EraVM` assembly generated from the yul code. It's a good idea to look at it to see the assembly that's actually going to run.
- `add.yul.zbin`: The bytecode to be run (i.e. the assembled version of the `zasm` file above).
- `programs_add.yul.runtime.optimized.ll`: The optimized LLVM IR generated by the compiler.
- `programs_add.yul.runtime.unoptimized.ll`: The unoptimized LLVM IR generated by the compiler.

## Running programs

After compiling, you can run any program by passing the binary file to the interpreter with:

```
cargo run -- <path_to_bin>
```

As an example, if you are running the `add.yul` program mentioned above, you need to run

```
cargo run -- program_artifacts/add.artifacts.yul/add.yul.zbin
```

## Documentation

Documentation can be found under the `docs` folder. Still a work in progress.

## Useful references

- [EraVM Primer](https://github.com/matter-labs/zksync-era/blob/main/docs/specs/zk_evm/vm_specification/zkSync_era_virtual_machine_primer.md). Highly recommended to read before diving into this codebase.
- [zksolc Compiler Documentation](https://github.com/matter-labs/zksync-era/blob/main/docs/specs/zk_evm/vm_specification/compiler/README.md). Very useful, as we are constantly interfacing with the compiler and its generated assembly.
- [More General VM Docs](https://github.com/matter-labs/zksync-era/tree/main/docs/specs/zk_evm)
- [EraVM Formal Spec](https://github.com/matter-labs/zksync-era/blob/main/docs/specs/zk_evm/vm_specification/EraVM_formal_specification.pdf)

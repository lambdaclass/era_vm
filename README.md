# EraVM

## Requirements

- [Rust](https://www.rust-lang.org/tools/install)
- [The EraVM Compiler: zksolc 1.5.1](https://github.com/matter-labs/zksolc-bin). Download the [latest binary](https://github.com/matter-labs/zksolc-bin/releases/tag/v1.5.1), then put it under your path. If done correctly, running `zksolc --version` should return 1.5.1.

## Testing

Run

```
make deps
```

to fetch the `era-compiler-tester` test suite and install all the zkSync LLVM dependencies.

Then do

```
make test
```

to run all tests.


## Documentation

Documentation can be found under the `docs` folder. Still a work in progress.

## Useful references

- [EraVM Primer](https://github.com/matter-labs/zksync-era/blob/main/docs/specs/zk_evm/vm_specification/zkSync_era_virtual_machine_primer.md). Highly recommended to read before diving into this codebase.
- [zksolc Compiler Documentation](https://github.com/matter-labs/zksync-era/blob/main/docs/specs/zk_evm/vm_specification/compiler/README.md). Very useful, as we are constantly interfacing with the compiler and its generated assembly.
- [More General VM Docs](https://github.com/matter-labs/zksync-era/tree/main/docs/specs/zk_evm)
- [EraVM Formal Spec](https://github.com/matter-labs/zksync-era/blob/main/docs/specs/zk_evm/vm_specification/EraVM_formal_specification.pdf)

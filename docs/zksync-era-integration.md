## Zksync-era integration

The `era_vm` doesn't do much by itself, it needs someone to tell it what to do and orquestrate it. On ethereum, for example, the execution node would take a block and process all of the transactions within it one by one using the `evm`. In zksync, things are a bit different, here transactions aren't processed one by one but in batch, this is to publish all of the transactions as just a single one, making the l1 post cheaper and distribitued among all the trnansactions withing the block.

This is what the bootloader do. The bootloader is a `.yul` contract that lives on l1 and takes an array of transactions and executes them in the `era_vm`.

Part of integrating our vm with the zk stack comes down to integrate the bootloader with our vm. That is, we need to make our vm "talk" with the bootloader. Before diving into the integration, we'll first explore what the bootloader does more in detail, which will explain what parts we need to keep track on our vm.

### Bootloader

At the most basic level, the bootloader performs the following steps:

1. Transaction Validation and Processing
2. State Initialization
3. Execution
4. Publishing final state to the l1.

## Transaction processing and validation

### How the bootloader communicates with the vm

In the `era_vm` there is one special heap that is reserver for the bootloader. Only the bootloader can write to that heap and in order to do that there are reserved memory address than when written will stop the vm execution and the return the written value(see [here]()). We call this hooks and they allow the bootloader to gather information and data after every transaction or even in the middle of it. Hooks are used a lot through the execution of transaction. The most important hooks are:

-   PubdataRequested
-   PostResult
-   NotifyAboutRefunds
-   AskOperatorForRefund
-   TxHasEnded

We'll dive deeper into the behaviour of these hooks, but first, you should know that there are two modes of execution:

-   OneTx: executes only one transaction.
-   Batch: executes an array of transaction and batches it as a single one to the l1.

Hooks will react based on the execution node.

Explain hooks.

### Rollbacks and snapshots

When executing a transaction under panics and reverts the changes have to be rollbacked, that is, we have to set them back as they were in the previous frame. Remember that frames get created under `near_call` and `far_call` [opcodes](). That is why, in the struct we see that the fields are marked as `Rollbackable`. Currently, we are handling rollbacks trough snapshots, which are simple copies of the current field state and then when rollbacking we change the state with the snapshot one. In the future, we probably want to include a more performant way of doing it(see [here]() for a current opened discussion).

The bootloader also has rollbacks, though they differ. Bootloader rollbacks involve a rollback to the whole bootloader state and the whole vm(the execution and the state). The latter are what we call `external snapshots`, this create a snapshot of whole state. [Here]() is some code. Before starting a new transaction, the bootloader crates an snapshot which it will use to make statistics to the last tx and it might rollback the whole vm if the whole transaction has failed. see [here]().

## Execution

Here the bootloader calls the `era_vm` to execute of a transaction.

## State initialization

At the start of every transaction, the bootloader loads the necessary contracts and prepares the environment for the era_vm. The `era_vm` receive a pointer to a struct that implements the following trait:

```rust
trait Storage: Debug {
    fn decommit(&mut self, hash: U256) -> Option<Vec<U256>>;

    fn storage_read(&mut self, key: &StorageKey) -> Option<U256>;

    fn cost_of_writing_storage(&mut self, key: &StorageKey, value: U256) -> u32;

    fn is_free_storage_slot(&self, key: &StorageKey) -> bool;
}
```

[Here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/vm.rs#L726) you can see the implementation of this function.

This storage is saved on the vm state and it is used all through the opcodes:

-   decommit: given a hash it returns a contract bytecode.
-   storage_read: given a key it returns the value from the initial contract storage
-   cost_of_writing_storage: when writing to the contract storage, gas is consumed, but the cost of writing is depends on wether the write is initial or not.
-   is_free_storage_slot: if the address to write belongs to a system contract and the key belongs to the bootloader address.

This functions are used to keep track of refunds and pubdata costs.

## Publishing final state

At the end of the batch, the bootloader needs to publish the pubdata to l1. Pubdata consists of the following fields:

-   L2 to L1 Logs: explain
-   L2 to L1 Messages: explain
-   Smart Contract Bytecodes: explain compression
-   Storage writes: explain store, storage keys, in diff.

[Here]() is an implementation.

This, requires the era_vm to keep the state. For that we hold this struct

```rust
struct VMState {}
```

You can see the implementation [here]().

Here is what each field represents:

-   storage_changes:
-   pubdata:
-   l2_to_l1_messages:
-   events:
-   refunds:
-   decommited_hashes:
-   ...:

For example, at the end of a batch, the bootloader will query the diff changes from the start of the tx, to publish it to ethereum. See [here]() for the implementation.

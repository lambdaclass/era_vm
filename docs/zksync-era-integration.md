## Introduction

So far we have been talking only about the `era_vm`. But you should know that the vm is only a small part in the zk stack. In fact, the zk stack is composed of many critical components. In this section we are only going to be interested in one particular component: the **nodes**, which can further be decomposed in the following units:

-   **Operator**: the server that initialises the vm, injects the bootloader bytecode, receives transactions and pushes them into the vm(bootloader memory to be more specific) and start batches and seals them.
-   **Bootloader**: a system contract that receives an array of transaction which are processed, validated, executed and then, the final state is published in the l1.
-   **era_vm**: the virtual machine where the bootloader(and so all the transactions bytecode) gets executed.

These three components are interacting with each other all the time to process transactions. In the next document, we'll go over general overview of the bootloader. Then, we'll move and do the same with the operator. After that, we'll see how the operators manages the bootloader. Finally, at the end, we'll see how data gets published on the l1. All of this, with the perspective of how this impacts the design of the `era_vm` which is what we care about the most in here.

## Bootloader

The bootloader is a special system contract whose hash lives on the l1 but its code isn't stored on the l1 nor on l2 but it gets compiled from `.yul` to the `era_vm` assembly with `zksolc` when the operator first initialises the vm(more on that below).

The bootloader, unlike ethereum, takes an array of transactions(a batch) and executes all of them in one run (unless specified not to, that is, if the execution mode of the vm is set to OneTx. More on this below). This approach allows for the batch of transaction to be then posted on the l1 as just a single one, making the processing on ethereum cheaper, since taxes and gas can be distributed among all the transactions within the posted batch.

At the most basic level, the bootloader performs the following steps:

1. Reads the initial batch information and make a call to the SystemContext contract to validate the batch.
 <!-- Here we could also add that the transaction gets processed based from where it came from (l1 or l2) -->
2. Loops through all transactions and executes them until the `execute` flag is set to $0$, at that point, it jumps to step 3.
3. Seals l2 block and publish final data to the l1.

[Here](https://github.com/matter-labs/era-contracts/blob/main/system-contracts/bootloader/bootloader.yul#L3962-L3965) you can see the main loop in the bootloader.

If you are curious and want to know more, here is the bootloader [contract](https://github.com/matter-labs/era-contracts/blob/main/system-contracts/bootloader/bootloader.yul) implementation.

## Operator

Currently the operator is a centralized server(there are plans to make a decentralised consensus see [here]) which can be thought as the entry point of the node, its responsibilities are:

-   Initializing the `era_vm` and keep its state.
-   Orchestrating the bootloader and keep its state.
-   Keeping storage database and commiting changes.

Here is a simplified version of what the vm on the operator end looks like:

```rust
struct OperatorVm {
    pub(crate) inner: EraVM, // this would be the actual `era_vm`
    pub suspended_at: u16, // last pc when execution stopped because of a hook
    pub bootloader_state: BootloaderState,
    pub(crate) storage: StorageDb,
    pub snapshot: Option<VmSnapshot>,
}
```

### Initializing the vm

The process of initializing the vm consists on:

-   Loading the bootloader bytecode and initializing its state.
-   Setting up the `era_vm` by injecting the bootloader code, loading default contracts, and setup other settings.

When setting up `era_vm`, the operator provides access to the chain storage database with the following API:

```rust
trait Storage {
    fn decommit(&mut self, hash: U256) -> Option<Vec<U256>>;

    fn storage_read(&mut self, key: &StorageKey) -> Option<U256>;

    fn cost_of_writing_storage(&mut self, key: &StorageKey, value: U256) -> u32;

    fn is_free_storage_slot(&self, key: &StorageKey) -> bool;
}
```

This storage is saved on the vm state as a pointer. Here is what each function does:

-   **decommit**: given a hash it returns a contract bytecode.
-   **storage_read**: given a key, it returns the potential value.
-   **cost_of_writing_storage**: when writing to the contract storage, gas is consumed, but the cost of writing is depends on whether the write is initial or not. More on that [here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/docs/specs/zk_evm/fee_model.md).
-   **is_free_storage_slot**: if the address to write belongs to the system context and the key belongs to base L2 token address, then the storage_slot is free(doesn't incur in gas charges).

A few notes about this storage:

1. The key has the following structure:

```rust
struct StorageKey {
    pub address: H160,
    pub key: U256,
}
```

And this allows us to query a key that belongs to the current executing address.

2. There isn't any consensus or spec about how storage should be implemented. We came up with this API because it is what we though was more convenient for the requirement. But, for example, the vm1 implements a query logic, where the operator will react based on the provided params.

[Here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/vm.rs#L726) you can take a look a the implementation of this trait in detail.

This functions are specially used in the `era_vm` to calculate refunds and pubdata costs. See [Here](https://github.com/lambdaclass/era_vm/blob/main/src/state.rs#L108-L123) and [here](https://github.com/lambdaclass/era_vm/blob/main/src/state.rs#L132-L173).

Finally, here is the [full](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/vm.rs#L79-L154) vm initialization code.

### Orchestrating the bootloader

The operator is responsible for managing the bootloader within the `era_vm`. This includes injecting the bootloader code into the `era_vm` and maintaining its state by:

-   Pushing transactions into the bootloader.
-   Passing necessary parameters into the bootloader's memory.
-   Start bootloader execution.
-   Rollback the vm state in case of an err.

Given that both transactions and the bootloader operate within the same `era_vm`, the bootloader has access to a reserved heap where the operator can write any required data. This interaction is continuous, as the bootloader is unaware of the broader state of the `era_vm`. To facilitate communication, the bootloader can write to a special address that triggers a suspension of the `era_vm` execution, allowing the operator to provide necessary data. This are known as hooks, and based on the written value a specific hook will get triggered by the operator. Hooks are extensively used throughout transaction execution, enabling the bootloader to gather information and data both after transactions and during their execution.Here are some of the most important hooks:

-   PostResult: Sets the last transaction result
-   TxHasEnded: if the mode of execution is set to **OneTx**, then the execution is stopped and it returns the result collected in the **PostResult** hook.
-   NotifyAboutRefunds: Informs the operator about the amount of gas refunded after a transaction.
-   AskOperatorForRefund: here the bootloader asks the operator to suggest a refund amount for the transaction.
-   PubdataRequested: At the end of the batch, the bootloader ask for the data to publish on the l1(more about this later).

Now, where does the operator know where to write to? Well, within the `era_vm`, there exists a special heap reserved exclusively for the bootloader. The Operator writes all the data in that heap which has designated slots based on the type of data to write(see more [here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/docs/specs/zk_evm/bootloader.md#structure-of-the-bootloaders-memory)). Transactions, for example, are pushed into the `[252189..523261]` slots.

### Rollbacks and snapshots

In the `era_vm`, when a transaction encounters a panic or reverts, the vm needs to roll back the changes, restoring only a part of the [state](https://github.com/lambdaclass/era_vm/blob/main/src/state.rs#L43-L60) to its previous frame. Remember that frames are created under `near_call` and `far_call` opcodes, and to manage state rollbacks, fields in the related structs are marked as `Rollbackable` (see [here](https://github.com/lambdaclass/era_vm/blob/zksync-era-integration-tests/src/state.rs#L299-L307)).

Currently, rollbacks are handled through snapshots by copying the current state of fields. If a rollback is necessary, the state is restored from these snapshots. While this method is functional, there’s ongoing discussion about implementing a more efficient rollback mechanism in the future (see [here](#)).

The Bootloader, can fail sometimes, and it is the job of the Operator to trigger rollbacks. Though this type of rollbacks differ from the former. Bootloader rollbacks involve restoring not only the vm state but the also the bootloader state and the whole vm(the execution and the state). This snapshots are called `external snapshots` and can only be triggered by the bootloader. [Here]() you can see what a full snapshot looks like. Before starting a new batch execution, the operator creates a snapshot. This snapshot is also used at the end of the execution to collect the logs.

> Notice that when we say vm `state`, we refer to the changes made to the data that lives on the chain, and the vm `execution` is the vm state of registers, memory, etc. This difference is important, since transactions reverts and panics only rollback the vm state (actually just a part of it not all), but a bootloader rollbacks also restore the vm `execution`.

## Publishing data

As said above, once the batch of transactions have all been executed, the final step in the bootloader is to publish the final data. The data to be published if composed of:

-   **L2 to L1 Logs**: Logs generated during L2 transactions that need to be recorded on L1. This can be transactions on L1 that have been forwarded to the L2 to lower costs.
-   **L2 to L1 Messages**: used to transmit instructions or data from smart contracts on L2 to contracts or systems on L1.
-   **Smart Contract Bytecodes**: This involves publishing the bytecodes of smart contracts deployed on L2. Before being sent to L1, these bytecodes are often compressed to save space and reduce costs.
-   **Storage writes**: These are records of changes to the storage on L2. Only the final diff from the previous state is included.

In theory, with this data one should be able to reconstruct the whole state of the l2.

At the end of the batch, bootloader calls the `PubdataRequested` hook to ask the operator for the final batch state. The operator writes into the bootloader memory(slots [40053..248052]) the collected data from the`era_vm`. [Here]() the hook implementation in detail.

Now, this requires the `era_vm` to keep a state for all the changes in the L2 state. For that we hold the following a struct:

```rust
struct VMState {
    storage_changes: HashMap<StorageKey, U256>,
    transient_storage: HashMap<StorageKey, U256>,
    l2_to_l1_logs: Vec<L2ToL1Log>,
    events: Vec<Event>,
    pubdata: Primitive<i32>,
    pubdata_costs: Vec<i32>,
    paid_changes: HashMap<StorageKey, u32>,
    refunds: Vec<u32>,
    read_storage_slots: HashSet<StorageKey>,
    written_storage_slots: HashSet<StorageKey>,
    decommitted_hashes: HashSet<U256>,
}
```

Here is what each field represents:

-   **storage_changes**: Tracks the changes to the storage keys and their new values during execution.
-   **transient_storage**: Temporary storage that last until the end of the transaction, meaning that it gets cleared after every transaction.
-   **l2_to_l1_logs**: Logs generated during execution that need to be sent from L2 to L1.
-   **events**: Events triggered during contract execution, often used for logging or triggering other actions.
-   -   **pubdata_costs**: The costs associated with publishing data to L1, used for fee calculations.
-   **pubdata**: Holds the sum of `pubdata_costs`.
-   **paid_changes**: After every write, tracks the cost to write to a key, to charge the difference in price on a subsequent writes to that key .
-   **refunds**: A list of refund amounts that have been calculated during the transaction.
-   **read_storage_slots**: A set of storage keys that have been read during execution, used to calculate gas fees and refunds.
-   **written_storage_slots**: A set of storage keys that have been written to during execution, used to calculate gas fees and refunds.
-   **decommitted_hashes**: Stores the hashes that have been the decommited through the whole execution. When decommiting a hash in a `far_call` or `decommit`, we check if the has been already decommited, if true then the decommit is free of charge.

And so we end up with two key structures on the `era_vm`:

-   The execution state: the state of registers, heaps, frames, etc.
-   The L2 state changes: the changes on the chain that will get publish on L1 and committed on the l2 database.

You can see the implementation and how we work with it [here](https://github.com/lambdaclass/era_vm/blob/zksync-era-integration-tests/src/state.rs).

Finally, everything finishes by the operator committing the changes to its database.

## Final comment

This document aims to give you a brief overview of the `era_vm` integration with the zk-stack and how this impacts the vm design and architecture. For that, we first needed to understood what a node is and its parts: bootloader and operator. In the explanation many details of the bootloader and operator where left behind, we only pick the parts that mostly involved and impacted the `era_vm` design.

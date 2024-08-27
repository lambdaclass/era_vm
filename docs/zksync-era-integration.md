# ZK stack

## Introduction

So far we have been talking only about the `era_vm`. But you should know that the vm is only a small part of the zk stack. The zk stack is composed of many critical components. In this section, we are only going to be interested in one particular component: the **operators**, which can further be decomposed into the following units:

-   **Operator/Sequencer**: the server that initializes the vm, injects the Bootloader bytecode, receives transactions and pushes them into the vm (Bootloader memory to be more specific) and start batches and seals them.
-   **Bootloader**: a system contract that receives an array of transactions which are processed, validated, executed, and then, the final state is published in the l1.
-   **era_vm**: the virtual machine where the Bootloader (and so all the transactions bytecode) gets executed.

These components interact continuously to process transactions. This document will provide an overview of the Bootloader, then explore the operator, its management of the Bootloader, and finally, the data publishing process to L1. All of this while primarily focusing on how these interactions impact the design of the `era_vm`.

## Bootloader

The Bootloader is a special system contract whose hash resides on L1, but its code isn't stored on either L1 or L2. Instead, it’s compiled from `.yul` to `era_vm` assembly using `zksolc` when the operator first initializes the VM (more on that below).

The Bootloader takes an array of transactions(a batch) and executes all of them in one run (unless specified not to, that is, if the execution mode of the vm is set to OneTx). This approach allows the transaction batch to be posted on the l1 as just a single one, making the processing on Ethereum cheaper, since taxes and gas can be distributed among all the transactions within the posted batch and data publishing costs can be reduced by posting only state diffs.

At the most basic level, the Bootloader performs the following steps:

1. Reads the initial batch information and makes a call to the SystemContext contract to validate the batch.
2. Loops through all transactions and executes them until the `execute` flag is set to `0`, at that point, it jumps to step `3`. <!-- Here we could also add that the transaction gets processed based on where it came from (l1 or l2) -->
3. Seals L2 block and publish final data to the l1.

//TODO
Note that the step `2` will depend on where the transaction came from.

The initial validation of the batch is necessary, since, as we'll see below, the Bootloader starts with its memory pre-filled with any data the operator wants. That is why it needs to validate its correctness.

For more details, you can see the [main loop](https://github.com/matter-labs/era-contracts/blob/main/system-contracts/bootloader/bootloader.yul#L3962-L3965) or the [full contract code](https://github.com/matter-labs/era-contracts/blob/main/system-contracts/bootloader/bootloader.yul).

## Operator/sequencer

Currently, the operator is a centralized server (there are plans to make a decentralized consensus for operators) which can be thought of as the entry point, its responsibilities are:

-   Initializing the `era_vm` and keeping its state.
-   Orchestrating the Bootloader and keeping its state.
-   Keeping storage database and committing changes.

Here is a simplified version of what the vm on the operator end looks like:

```rust
struct OperatorVm {
    pub(crate) inner: EraVM, // this would be the actual `era_vm`
    pub suspended_at: u16, // last pc when execution stopped because of a hook
    pub Bootloader_state: BootloaderState,
    pub(crate) storage: StorageDb,
    pub snapshot: Option<VmSnapshot>,
}
```

### Initializing the vm

VM initialization involves:

-   Loading the Bootloader bytecode and initializing its state.
-   Setting up the `era_vm` by injecting the Bootloader code, loading default contracts, and configuring other settings.

When setting up `era_vm`, the operator provides access to the chain storage database with the following API:

```rust
trait Storage {
    fn decommit(&mut self, hash: U256) -> Option<Vec<U256>>;

    fn storage_read(&mut self, key: &StorageKey) -> Option<U256>;

    fn cost_of_writing_storage(&mut self, key: &StorageKey, value: U256) -> u32;

    fn is_free_storage_slot(&self, key: &StorageKey) -> bool;
}
```

This storage is saved in the VM state as a pointer. Here’s a brief explanation of each function:

-   **decommit**: given a contract hash it returns its corresponding bytecode (if it exists) from the database.
-   **storage_read**: given a key, it returns the potential value from the database.
-   **cost_of_writing_storage**: when writing to the contract storage, gas is consumed, but the cost of writing depends on whether the write is initial or not. More on that [here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/docs/specs/zk_evm/fee_model.md).
-   **is_free_storage_slot**: if the address to write belongs to the system context and the key belongs to the base L2 token address, then the storage_slot is free(doesn't incur gas charges).

A few notes about this storage:

1. The key has the following structure:

```rust
struct StorageKey {
    pub address: H160,
    pub key: U256,
}
```

This allows us to query a storage key belonging to any desired contract through its address.

2. There isn't any consensus or spec about how storage should be implemented. We came up with this API because it is what we thought was more convenient for the requirement. But, for example, the vm1 implements a query logic, where the operator will react based on the [provided params](https://github.com/matter-labs/zksync-era/blob/87768755e8653e4be5f29945b56fd05a5246d5a8/core/lib/types/src/zk_evm_types.rs#L17-L30).

[Here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/vm.rs#L726) you can take a look at the implementation of this trait in detail.

These functions are specially used in the `era_vm` to calculate refunds and pubdata costs. See [here](https://github.com/lambdaclass/era_vm/blob/main/src/state.rs#L108-L123) and [here](https://github.com/lambdaclass/era_vm/blob/main/src/state.rs#L132-L173).

Finally, here is the [full](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/vm.rs#L79-L154) vm initialization code.

### Orchestrating the Bootloader

The operator is responsible for managing the Bootloader within the `era_vm`. This includes injecting the bootloader code into the `era_vm` and maintaining its state by:

-   Pushing transactions into the Bootloader.
-   Passing necessary parameters into the Bootloader's memory.
-   Starting Bootloader execution.
-   Rolling back the VM state in case of errors.

Since both transactions and the Bootloader run in the same `era_vm`, the bootloader accesses a reserved heap where the operator writes any required data. This interaction is continuous, as the bootloader is unaware of the broader state of the `era_vm`. To facilitate communication, the bootloader can write to a special address that triggers a suspension of the `era_vm` execution, allowing the operator to provide necessary data. These are known as **hooks**, and based on the written value a specific hook will get triggered by the operator. Here are some of the most important hooks:

-   **PostResult**: sets the last transaction result
-   **TxHasEnded**: If the mode of execution is set to **OneTx**, then the execution is stopped and it returns the result collected in the _PostResult_ hook.
-   **NotifyAboutRefunds**: Inform the operator about the amount of gas refunded after a transaction.
-   **AskOperatorForRefund**: Here the Bootloader asks the operator to suggest a refund amount for the transaction.
-   **PubdataRequested**: At the end of the batch, the Bootloader asks for the data to publish on the l1 (more on this later).

Now, where does the operator know where to write to? Again, within the `era_vm`, there exists a special heap reserved exclusively for the Bootloader. The Operator writes all the data in that heap which has designated slots based on the type of data to write (see more [here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/docs/specs/zk_evm/bootloader.md#structure-of-the-bootloaders-memory)). Transactions, for example, are pushed into the `[252189..523261]` slots.

### Rollbacks and snapshots

In the `era_vm`, when a transaction encounters a panic or reverts, the vm needs to roll back the changes, restoring only a part of the [state](https://github.com/lambdaclass/era_vm/blob/main/src/state.rs#L43-L60) to its previous frame. Remember that frames are created under `near_call` and `far_call` opcodes. Currently, rollbacks are perform using snapshots which are just copies of the current state. If a rollback is necessary, the state is restored from these snapshots.

The Bootloader may fail sometimes, it is the job of the Operator to trigger the necessary rollbacks. However, this type of rollback differs from the ones just mentioned. Bootloader rollbacks involve restoring not only the [full vm state](https://github.com/lambdaclass/era_vm/blob/zksync-era-integration-tests/src/vm.rs#L66-L69) but also the bootloader state. These snapshots are called external snapshots and can only be triggered by the Bootloader. [Here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/snapshot.rs) you can see what a full snapshot looks like. Before starting a new batch execution, the operator creates a snapshot, which is also used at the end of execution to collect logs (see [here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/vm.rs#L508-L595) for more details).

> Notice that when we say vm `state`, we refer to the changes made to the data that lives on the chain, and the vm `execution` is the vm state of registers, memory, etc (see more [here](#era_vm-key-structures)). This difference is important since transactions reverts and panics only rollback the vm `state` (actually just a part of it not all), but Bootloader rollbacks also restore the vm `execution`.

## Publishing data

As said above, once the batch of transactions has all been executed, the final step in the Bootloader is to publish the final data. The data to be published is composed of:

-   **L2 to L1 Logs**: Logs generated during L2 transactions that need to be recorded on L1. This can be transactions on L1 that have been forwarded to the L2 to lower costs.
-   **L2 to L1 Messages**: used to transmit instructions or data from smart contracts on L2 to contracts or systems on L1.
-   **Smart Contract Bytecodes**: This involves publishing the bytecodes of smart contracts deployed on L2. Before being sent to L1, these bytecodes are often compressed to save space and reduce costs.
-   **Storage writes**: These are records of changes to the storage on L2. Only the final diff from the previous state is included.

In theory, with this data one should be able to reconstruct the whole state of the l2.

At the end of the batch, the Bootloader calls the `PubdataRequested` hook to ask the operator for the final batch state. The operator writes into the bootloader memory(slots [40053..248052]) the collected data from the`era_vm`. [Here](https://github.com/lambdaclass/zksync-era/blob/era_vm_integration_v2/core/lib/multivm/src/versions/era_vm/vm.rs#L309-L350) you can see the hook implementation in detail.

Now, this requires the `era_vm` to keep a state for all the changes in the L2 state. For that, we hold the following structure:

```rust
struct VMState {
    storage_changes: HashMap<StorageKey, U256>,
    transient_storage: HashMap<StorageKey, U256>,
    l2_to_l1_logs: Vec<L2ToL1Log>,
    events: Vec<Event>,
    pubdata_costs: Vec<i32>,
    pubdata: Primitive<i32>,
    paid_changes: HashMap<StorageKey, u32>,
    refunds: Vec<u32>,
    read_storage_slots: HashSet<StorageKey>,
    written_storage_slots: HashSet<StorageKey>,
    decommitted_hashes: HashSet<U256>,
}
```

Here is what each field represents:

-   **storage_changes**: Tracks the changes to the storage keys and their new values during execution.
-   **transient_storage**: Temporary storage that lasts until the end of the transaction, meaning that it gets cleared after every transaction.
-   **l2_to_l1_logs**: Logs generated during execution that need to be sent from L2 to L1.
-   **events**: Events triggered during contract execution.
-   **pubdata_costs**: The costs associated with publishing data to L1, used for fee calculations.
-   **pubdata**: Holds the sum of `pubdata_costs`.
-   **paid_changes**: After every write, tracks the cost to write to a key, to charge the difference in price on a subsequent writes to that key.
-   **refunds**: A list of refund amounts that have been calculated during execution.
-   **read_storage_slots**: A set of storage keys that have been read during execution, used to calculate gas fees and refunds.
-   **written_storage_slots**: A set of storage keys that have been written to during execution, used to calculate gas fees and refunds.
-   **decommitted_hashes**: Stores the hashes that have been the decommited through the whole execution.

> Note on `storage_change`: The bootloader requires storage changes to be sorted by address first and then by key. This sorting is essential because, before publishing the data, the bootloader invokes a contract called Compressor to compress the state diff and validate it. During validation, the Compressor receives both the compressed diff and the original state diff. The compression process sorts the map automatically, so then when the verification process starts, the provided original state diff must also be sorted. You can find the full validation function in the Compressor contract [here](https://github.com/matter-labs/era-contracts/blob/8670004d6daa7e8c299087d62f1451a3dec4f899/system-contracts/contracts/Compressor.sol#L77-L190) if you're interested. As we continue developing the VM, we're considering implementing a new data structure that maintains this order upon insertion, potentially avoiding the costly sorting process when dealing with large lists of changes.

<a id="era_vm-key-structures"></a>
And so we end up with two key structures on the `era_vm`:

-   [The execution state](https://github.com/lambdaclass/era_vm/blob/main/src/execution.rs#L32-L51): the state of registers, heaps, frames, etc.
-   The L2 state changes: the changes on the chain that will get published on L1 and committed to the l2 database.

Finally, everything finishes with the operator committing the changes to its database.

### Refunds, Storage write/read and Pubdata Costs associated

We keep several fields in the `VMState` to track refunds and pubdata associated costs. Refunds are a return of ergs spent, since gas is always paid upfront. They can occur during storage operations, specifically when:

-   Reading from storage
-   Writing to storage

The relevant opcodes for these operations are:

-   `far_call`
-   `decommit`
-   `sstore`
-   `ssload`

#### How Refunds Are Calculated

Refunds depend on whether a storage key has been accessed before. To manage this, we use the following keys from the `VMState`:

-   `paid_changes`
-   `read_storage_slots`
-   `written_storage_slots`
-   `decommited_hashes`

#### Decommit behaviour

Decommits might occur during `far_call` or `decommit`. Whenever we decommit a `hash`, we check if that hash has already been decommited, if it is then we return the gas spent for deommit since decommits are paid upfront. Otherwise, we store the hash a already decommited so subsequent decommits to that hash will become free of charge.

#### Storage Read Behavior

When performing a storage_read, we check if the slot is free or if the key has already been read. If the key has been read before, a "warm" refund is given. If not, no refund is provided, but the key is marked as read for future refunds.

#### Storage Write Behavior

During a storage_write, we first check if the slot is free. If it is, a "warm" refund is given. Otherwise, we calculate the pubdata cost—the current price for writing to storage. If the key has been written to before, we only pay the difference between the new price and the previously paid amount (this difference is what we track as `pubdata_costs`). This difference can be negative, resulting in a refund. Additionally, if the key has been written to before, a "warm" refund is provided. If the key has only been read before and is now being written to, a "cold" write refund is given.

#### What Defines a Free Slot?

The operator determines whether a slot is considered "free." This decision is based on whether the key address belongs to the system context contract or if it belongs to the L2_BASE_TOKEN_ADDRESS and is associated with the ETH bootloader’s balance.

[Here](https://github.com/lambdaclass/era_vm/blob/zksync-era-integration-tests/src/state.rs) is the full code on how we manage the state changes, refunds, pubdata and more.

## Final comment

This document provides an overview of the `era_vm` integration within the zk-stack, focusing on the Bootloader, and operator, and how their interactions impact the VM's design and architecture. In the explanation many details of the bootloader and operator were left behind, we only picked the parts that mostly involved and impacted the `era_vm` design.

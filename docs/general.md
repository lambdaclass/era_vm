# General documentation

## Heaps/Aux Heaps and Fat Pointers.

Heap is a bounded memory region to store data between near calls, and to communicate data between contracts.

Accessing an address beyond the heap bound leads to heap growth: the bound is adjusted to accommodate this address. The difference between old and new bounds is paid in gas.

Most instructions can not use heap directly. Instructions `ld.1` and `st.1` are used to load and store data on heap:

```asm
; take a 32-bit number from r1, use it as an offset in heap,
; load the word from heap by this offset to r4
ld.1 r1, r4

; take a 32-bit number from r3, use it as an offset in heap,
; store the word from r5 to heap by this offset
st.1 r3, r5
```

Heap is byte-addressable, but reads and writes operate in words. To read two consecutive words in heap starting at an address A, first, read from A, and then read from A+32. Reading any addresses in between is valid too.

One of the modifiers allows to immediately form a new offset like that:

```asm
; same as ld, but additionally r5 <- r1 + 32
ld.1.inc r1, r4, r5
```

This allows reading several consecutive words in a row:

```asm
; reads four consecutive words from heap starting at address in r8
; into registers r1, r2, r3, r4
ld.1.inc r8, r1, r8
ld.1.inc r8, r2, r8
ld.1.inc r8, r3, r8
ld.1.inc r8, r4, r8
```

In zkEVM, there are two heaps; every far call allocates memory for both of them.

Heaps are selected with modifiers `.1` or `.2` :

- `ld.1` reads from heap.
- `ld.2` reads from auxheap.

The reason why we need two heaps is technical. Heap contains calldata and returndata for calls to user contracts, while auxheap contains calldata and returndata for calls to system contracts. This ensures better compatibility with EVM as users should be able to call zkEVM-specific system contracts without them affecting calldata or returndata.

All heaps are stored in a vector and accessed via heap page IDs. When the program is loaded, three heaps are created: the primary heap with page ID 2, the auxheap with page ID 3, and a special calldata heap with page ID 1. Each time a far call is executed, new primary heap and auxheap are created. For calls to normal contracts, the calldata heap references the caller's primary heap. For calls to a system contract, the calldata heap references the caller's auxheap.

Apart from using opcodes `ld.1` and `ld.2`, heaps can also be accessed through the `FatPointerRead` operation, which is aliased as `ld`.

> [!NOTE]
> A `FatPointer` is a 4-tuple `(page,start,length,offset)` where the page indicates which heap it points to.

The `ld` opcode receives a Fat Pointer as input, and loads a 32 byte word of the correspondent heap starting at `start + offset`. If the length is smaller than 32 bytes, it fills the rest with 0s.

The `start` and `offset` fields seem like the same thing, but they differentiate when applying the concept of pointer narrowing.

Narrowing a pointer does the following:

```
new_start = start + offset
new_length = length - offset
new_offset = 0
```

When a far call is performed, the calldata heap is selected via a fat pointer, that we later store on register r1 for the new context to access.

There is no way of modifying heaps via Fat Pointers, they can only be used to read them.

## Far Calls vs Near Calls. CallFrames and Context

Far Calls are the equivalent of calls in the EVM, they are used to call external contracts. Near Calls are used to call internal functions within the same contract that is being executed.

Contracts have their own unique `Context` which itself can hold multiple `CallFrame`s. `CallFrame`s are used to keep track of the current state of the contract being executed.

#### When a Far Call is made, a new `Context` is created and pushed into the running `Context`s of the vm. `Context`s are composed of:

- Contract `Address`
- Caller `Address`
- Code `Address`
- Code Page
- `Stack`
- Running `CallFrame`s (created by Near Calls)
- `Heap`
- `AuxHeap`
- `CalldataHeap`

The amount of gas that can be allocated to a new `Context` is limited to 63/64 of the currently available gas in the running `Callframe`.

**A new Near Call will inherit the properties of the current `CallFrame`, and make use of the `Stack` and `Heap`s of the running `Context`**.

#### `CallFrame`s are composed of:

- Available gas
- Exception handler
- Stack Pointer
- Program Counter

### Far Call wrapping

Let's look at the following solidity code:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract LibraryContract {
    function someFunction() public {}
}

contract CallerContract {
    LibraryContract libraryContract;

    constructor(address _libraryContractAddress) {
        libraryContract = LibraryContract(_libraryContractAddress);
    }

    function callNonReturningFunction() public {
        libraryContract.someFunction();
    }
}
```

Here we are calling a function of an external contract, this should give us a `far_call` instruction in our compiled code.

This is part of the assembly compiled with `zksolc`:

```assembly
.text
	.file	"test.sol:CallerContract"
	.globl	__entry
__entry:
.func_begin0:
	nop	 stack+=[1 + r0]
	add	 r1, r0, r3
	shr.s	96, r3, r3
   ...
	near_call	r0, @__farcall, @DEFAULT_UNWIND
   ...
.func_end0:

__farcall:
.func_begin2:
.tmp0:
	far_call	r1, r2, @.BB2_2
.tmp1:
	add	 1, r0, r2
	ret
.BB2_2:
.tmp2:
	add	 r0, r0, r2
	ret
.func_end2:
...
```

Notice how instead of calling `far_call` directly, we are calling `near_call` which in turn calls `far_call`. This is because `far_call` does not discern whether the call was a success or not, so we need to wrap it in a `near_call` to return this boolean.

### Data passing between contracts

We make use of Fat Pointers to send and receive (read only) data between contracts, when choosing how to pass data to a contract (whether when calling or returning from a call) we have a choice:

- pass an existing fat pointer
- create a new fat pointer from a fragment of heap/auxheap.

This is handled by the `get_forward_memory_pointer`, which (respectively) narrows the pointer it receives, or creates a new one in the requested heap.

A Fat Pointer will delimit a fragment accessible to another contract. Accesses outside this fragment through a pointer yield zero. They also provide an offset inside this fragment, which can be increased or decreased.

## Call Types

Regular, Mimic, and Delegate. Explain the differences between them and how MimicCalls are used to call constructors on behalf of the deployed contract.

## Precompiles and System calls

Explain how calls to precompiles work, use keccak as an example as it's used on every deployment (it goes to the `keccak.yul` contract which then uses the `precompile` opcode).
What are system contracts? What's a system call? Show some examples (deployer, nonce holder, L2BaseToken) and what they're used for.

## `context.get_context_u128` opcode, msg.value, payable functions

In EraVM, `msg.value` is mapped to a 128-bit context value (`register_context_u128`), an essential part of the VMState.

> [!NOTE]
> In Solidity, `msg.value` represents the amount of ether (in wei) sent with a transaction.

This value can be accessed using the `SetContextU128` opcode, which reads the value stored in the specified register. During a `far_call`, the value is captured in the new `Context` as `context_u128`. To access this value, the `GetContextU128` opcode is used.

This value is set to zero in the `VMState` in the following cases:
- When a `far_call` is performed.
- When a panic or a revert occurs.
- When the `far_call` is completed.

If the function is non-payable and `context.get_context_u128` is not zero, the contract will revert. Here is an example:

```solidity
// SPDX-License-Identifier: MIT

pragma solidity >=0.4.16;

contract Test {
    function first() public pure returns(uint64) {
        uint64 result = 42;
        return result;
    }
    ...
}
```

The generated assembly for this contract is as follows:

```assembly
    .text
	.file	"default.sol:Test"
	.globl	__entry
__entry:
.func_begin0:
	add	128, r0, r3
	st.1	64, r3
	and!	1, r2, r0
	jump.ne	@.BB0_1
    ...
.BB0_1:
	context.get_context_u128	r1
	sub!	r1, r0, r0
	jump.ne	@.BB0_2
    ...
.BB0_2:
	add	r0, r0, r1
	ret.revert.to_label	r1, @DEFAULT_FAR_REVERT
    ...
```

In this example, the function is non-payable. After calling the constructor (block `@.BB0_1`), it checks if `context.get_context_u128` is zero. If not, it means wei is being passed, and the transaction will revert (jumping to the `.BB0_2` block) because the function is not `payable`.

Here's an example of a contract with a `payable` function:

```solidity
// SPDX-License-Identifier: MIT

pragma solidity >=0.8.8;

contract SendMoney {
    function sendMoney(address payable to) public payable {
        (bool success,) = to.call{value: msg.value}("");
        require(success, "Failed to send Ether");
    }
}
```
Let's compile this contract and examine the generated assembly for the  `sendMoney` function:

```assembly
	.text
	.file	"send_money.sol:SendMoney"
	.globl	__entry
__entry:
.func_begin0:
	add	128, r0, r3
	st.1	64, r3
	and!	1, r2, r0
    ...
.BB0_9:
	context.get_context_u128 r3 // retrieves msg.value
	...
	jump.ne	@.BB0_14
    ...
.BB0_14:
	or	@CPI0_5[0], r1, r1
	add	32777, r0, r2 // MsgValueSimulator contract address
	add	r0, r0, r5
.BB0_13:
    // Calls the `MsgValueSimulator` contract
	near_call	r0, @__farcall, @DEFAULT_UNWIND
    ...
```
The `context.get_context_u128` opcode is used to retrieve the value of `msg.value`, and the `MsgValueSimulator` contract (address `0x8009`, `32777` in decimal) is called. This contract simulates transactions with `msg.value` inside EraVM, transferring the value to the destination address.

The `MsgValueSimulator` contract is the following:

```solidity
// SPDX-License-Identifier: MIT

pragma solidity 0.8.20;
...

contract MsgValueSimulator is ISystemContract {
    ...
    fallback(bytes calldata _data) external onlySystemCall returns (bytes memory) {
        ...
        if (value != 0) {
            // Calls `L2BaseToken` contract to transfer the value
            (bool success, ) = address(REAL_BASE_TOKEN_SYSTEM_CONTRACT).call(
                abi.encodeCall(REAL_BASE_TOKEN_SYSTEM_CONTRACT.transferFromTo, (msg.sender, to, value))
            );
            if (!success) {
                assembly {
                    revert(0, 0)
                }
            }
            ...
        }
        ...
    }
}
```
The `MsgValueSimulator` contract transfers the value to the specified address by calling the `L2BaseToken` contract.


## Tracers and how to add prints

A `Tracer` should comply with the following trait

```
pub trait Tracer {
    fn before_execution(&mut self, _opcode: &Opcode, _vm: &mut VMState) -> Result<(), EraVmError>;
}
```

The `before_execution` function will be called on every loop just before the opcode execution.
Right now that is the only function the trait has, in the future more may be added as needed, like `before_decoding`, `after_decoding` or `after_execution`

An important Tracer is what we call the `PrintTracer`, with it we can print stuff on solidity contracts.

Here is an example of a contract with prints

```
pragma solidity >=0.4.16;

contract WithPrints {

    // This is for strings
    function printIt(bytes32 toPrint) public {
        assembly {
            function $llvm_NoInline_llvm$_printString(__value) {
                let DEBUG_SLOT_OFFSET := mul(32, 32)
                    mstore(add(DEBUG_SLOT_OFFSET, 0x20), 0x00debdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdf)
                    mstore(add(DEBUG_SLOT_OFFSET, 0x40), __value)
                    mstore(DEBUG_SLOT_OFFSET, 0x4A15830341869CAA1E99840C97043A1EA15D2444DA366EFFF5C43B4BEF299681)
        }
            $llvm_NoInline_llvm$_printString(toPrint)
        }
    }
    // This is for numbers
   function printItNum(uint256 toPrint) public {
        assembly {
            function $llvm_NoInline_llvm$_printString(__value) {
                let DEBUG_SLOT_OFFSET := mul(32, 32)
                    mstore(add(DEBUG_SLOT_OFFSET, 0x20), 0x00debdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebdebde)
                    mstore(add(DEBUG_SLOT_OFFSET, 0x40), __value)
                    mstore(DEBUG_SLOT_OFFSET, 0x4A15830341869CAA1E99840C97043A1EA15D2444DA366EFFF5C43B4BEF299681)
        }
            $llvm_NoInline_llvm$_printString(toPrint)
        }
    }

    function aFunction() public returns(uint64) {
        uint64 result = 42;
        printIt("RESULT");
        printItNum(result);
        return result;
    }
}
```

There are two types of prints, strings and numbers, for that we have printIt and printItNum respectively.
What these functions are doing is, they use a debug slot defined as 1024, on `debug_slot + 32` we store a value that indicates if the print is going to be a string or a number, then on `debug_slot + 64` we store the value itself, and on `debug_slot` we store a magic value.

Here is where the PrintTracer does its magic, before every execution it looks if the opcode executed is a `HeapWrite`, this is the opcode responsible for storing things in the Heap, which is in the end what we are doing with the `mstore`, if the value being written is the magic value and its being written on the debug slot, then we know we are in one of the print functions and we need to print the value.

So we get from the heap the values on `debug_slot + 32` and `debug_slot + 64`, with the first one we check if we have to print a string or a number, and we print the correspondent one.

Have in mind that currently, mainly because of compiler optimizations, some prints may not appear, specially if they are right after each other.

In order to perform prints, at the moment we need to change the following on `src/lib.rs`

```
fn run_opcodes(vm: VMState, storage: &mut dyn Storage) -> (ExecutionOutput, VMState) {
    run(vm.clone(), storage, &mut []).unwrap_or((ExecutionOutput::Panic, vm))
}
```

For

```
use tracers::print_tracer::PrintTracer;

...

fn run_opcodes(vm: VMState, storage: &mut dyn Storage) -> (ExecutionOutput, VMState) {
    let mut tracer = PrintTracer {};
    run(vm.clone(), storage, &mut [Box::new(&mut tracer)]).unwrap_or((ExecutionOutput::Panic, vm))
}
```

## Difference between a revert and a panic; exception handlers

There are three ways to end execution: `Ok`, `Revert`, and `Panic`.

- `Ok` indicates that the current call ended correctly.
- `Revert` and `Panic` both signal that the call did not end correctly.

### Differences Between `Revert` and `Panic`

- **Gas Refund:** `Revert` returns unspent gas to the caller, while `Panic` does not.
- **Return Value:** `Revert` can include a return value, whereas `Panic` cannot.
- **Flags:** `Panic` sets the overflow (OF) flag.

### Execution Methods

- **Panic:** Executed via the `ret.panic` opcode.
- **Revert:** Can be executed via the `ret.revert` opcode, an error within the VM, or running out of gas.

### Instruction Flow

- **Ok:** After an `Ok` execution, the next instruction is the one immediately following the call.
- **Revert and Panic:** The next instruction is determined by the exception handler, defined in the near or far call, and stored in the corresponding `CallFrame`.


For example in the following code

```asm
__entry:
add 5,r0,r1
near_call r0,@call,@exception
add 6,r0,r1

__call:
add 7,r0,r1
ret.ok

__exception:
add 8,r0,r1
```

The following instructions will be executed

```asm
add 5,r0,r1
near_call r0,@call,@exception
    add 7,r0,r1
    ret.ok
add 6,r0,r1
```

But if it were like this

```asm
__entry:
add 5,r0,r1
near_call r0,@call,@exception
add 6,r0,r1

__call:
add 7,r0,r1
ret.revert

__exception:
add 8,r0,r1

```

It would execute

```asm
add 5,r0,r1
near_call r0,@call,@exception
    add 7,r0,r1
    ret.revert
add 8,r0,r1

```

## Bootloader

Operator execution (transactions come in, get executed on the bootloader, state is suspended until new transaction shows up).

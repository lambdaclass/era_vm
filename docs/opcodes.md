## General Info
Opcodes have variants: 11 bits specifying the type of opcode. Variants:
- `Invalid`
- `Nop`
- `Add`
- `Sub`
- `Mul`
- `Div`
- `Jump`
- `Context`
- `Shift`
- `Binop`
- `Ptr`
- `NearCall`
- `Log`
- `FarCall`
- `Ret`
- `UMA`

Some of these variants internally have more subtypes. For example, the `Log` opcodes can be
reads or writes to storage, memory, etc. To know the opcode from the `11` bits given, an `opcode_table` is used. Generating the table is pretty complex, and it's part of the reason why the separate Matter Labs repo `era-zkevm_opcode_defs` exists.

We would like to stop using it, but for now we depend on it, as it was pretty hard to extract the table itself (it'd be easier if it was a `const fn` but it's a `lazy_static` instead). Note that even though variants are 11 bits (i.e. 2048 possibilities), most of them are invalid opcodes. There aren't actually 2048 valid instructions.

Opcodes may have two source (`src0` and `src1`) and two destination (`dst0` and `dst1`) operands.
`src1` and `dst1` are always registers, while `src0` and `dst0` can be of different types, which are:
- `RegOnly`
- `RegOrImm(RegOrImmFlags)`:
    - `UseRegOnly`
    - `UseImm16Only`
- `Full(ImmMemHandlerFlags)`:
    - `UseRegOnly`
    - `UseStackWithPushPop`
    - `UseStackWithOffset`
    - `UseAbsoluteOnStack`
    - `UseImm16Only`
    - `UseCodePage`

Each opcode also has two possible flags that can be set:
- `SWAP_OPERANDS_FLAG_IDX_FOR_ARITH_OPCODES`
- `SWAP_OPERANDS_FLAG_IDX_FOR_PTR_OPCODE`

These flags are meant for swapping operands on some opcodes. The goal here is that sometimes you may want to have the second operand (e.g. `src1`) be on the stack, which is not allowed. For this you swap the operands.

Every opcode can be predicated, meaning they are only executed if a certain condition is met. Possible predicates are
- `Always`
- `Gt`
- `Lt`
- `Eq`
- `Ge`
- `Le`
- `Ne`
- `GtOrLt`

## Opcode Encoding

- The first `11` bits are the variant. The value can be looked up in an opcode table to know the opcode.
- The condition (predicate) is encoded by bits `14`, `15` and `16`. To get them you mask against `0xe000`, then shift right 13 times.
- Bits `17` through `24` have the `src0_index` as the first 4 bits and the `src1_index` as the last 4 bits.
- Bits `25` through `32` have the `dst0_index` as the first 4 bits and the `dst1_index` as the last 4 bits.
- Bits `33` through `48` are `imm0` (2 bytes, a `u16`)
- Bits `49` through `64` are `imm1` (2 bytes, a `u16`)

Note that there are actually two different encodings in zkSync: one for testing and one for production. The one described here is the production one; the testing one does not concern us right now.

Values in registers **ARE TAGGED**. They carry with them the knowledge of whether they are pointers or not.
This is what the `zkSync` VM calls a `PrimitiveValue`. We call it a `TaggedValue` instead.

## Bytecode example
Below is an example of a simple program compiled to bytecode, with the corresponding assembly on top of each instruction. Note that they are represented in little endian, so the variant is **at the end**, not the beginning (so for example, the value for the `ADD` with immediate instruction is `0x39`, the value for `sstore` is `0x41b` and so on).

```
// add	2, r0, r1
0000000201000039
// sstore	r0, r1
000000000010041b
// add	1, r0, r2
0000000102000039
// sstore	r2, r1
000000000012041b
// sstore	r1, r1
000000000011041b
// add	3, r0, r2
0000000302000039
// sstore	r2, r1
000000000012041b
// add	r0, r0, r1
0000000001000019
// ret
000000000001042d
// I still don't know exactly what this code below that the compiler generated is for. The first instructions are additional ret.ok or ret.panic ones; the rest I don't know. The last four ones look more like a hash to me than actual code (it would make sense as well since together they are 256 bits).
0000000900000432
0000000a0001042e
0000000b00010430
0000000000000000
0000000000000000
0000000000000000
0000000000000000
04e50e9e3e2c8d56
cb381096acaffaff
e2bc853833eefedc
f05db2cd97ac121b
```

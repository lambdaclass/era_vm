This is a python script for generating arbitrary asm files.

It should be invoked like this:

`python3 generate_asm.py <path_to_your_asm_file>`

The asm given should have the following format:

```nasm
	.text
	.file	"test.zasm"
	.globl	__entry

__entry:
.func_begin0:
	add r0,r0,stack+=[5]
	; %2=r9,stack[1]
	add 5, r0, %2
	; %2=r9,stack[1] ; %3=r11,stack[3]
	add %2, r0, %3
	; %3=r11,stack[3]
	sstore	r0, %3
	ret
.func_end0:

	.note.GNU-stack
	.rodata
```
Where the `%x` , x being any integer, represents the position in the file that you want to change

Let's take as an example this two lines
```nasm
        ; %2=r9,stack[1]
	add 5, r0, %2
```
Here `%2` in the second line is whats going to be replaced, and the comment (marked with ; ) tells you what is going to be replaced for.

In this case, there are 2 values `r9` and `stack[1]` separated by a comma, this means two different files are going to be generated, one with `r9` and another with `stack[1]`.

The same happens with any other `%x`, and if there are several instances of the same `%x` they don't generate extra files, this means, in the following example:
```nasm
        ; %2=r9,stack[1]
	add 5, r0, %2
        ; %2=r9,stack[1]
	add %2, r0, r11
```
Only 2 files will be generated:
```nasm
        ; %2=r9,stack[1]
	add 5, r0, r9
        ; %2=r9,stack[1]
	add r9, r0, r11
```
And
```nasm
        ; %2=r9,stack[1]
	add 5, r0, stack[1]
        ; %2=r9,stack[1]
	add stack[1], r0, r11
```

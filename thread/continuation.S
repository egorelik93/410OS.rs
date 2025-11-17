//! Assembly functions for manipulating control flow.

.global exitKernelMode
exitKernelMode:
        pop %eax

        popa

        pop %gs
        pop %fs
        pop %es
        pop %ds

        iret


.global callWithCurrentContinuation
.global continueFromContinuation

callWithCurrentContinuation:
        push %ebp
        mov %esp, %ebp
        pusha
        mov %esp, %esi
        push 0xC(%ebp)
        push %esi
        call 0x8(%ebp)
        push %eax
continueFromContinuation:
        push %ebp
        mov %esp, %ebp
        mov 0x8(%ebp), %esp
        popa
        pop %ebp
        ret

.macro SAVE_REGS
    // Allocate space on stack for TrapFrame (17 words)
    sub sp, sp, #68
    
    // Save r0-r12
    stmia sp, {{r0-r12}}
    
    // Save user sp, lr, and return address
    str sp, [sp, #52]  // Save user sp (will be adjusted)
    str lr, [sp, #56]  // Save lr
    
    // For exception return, we need to adjust the return address
    // and save the original PC
    sub r0, lr, #4     // Adjust return address
    str r0, [sp, #60]  // Save PC
    
    // Save CPSR
    mrs r0, cpsr
    str r0, [sp, #64]
.endm

.macro RESTORE_REGS
    // Restore CPSR
    ldr r0, [sp, #64]
    msr cpsr_cxsf, r0
    
    // Restore PC to lr for return
    ldr lr, [sp, #60]
    
    // Restore r0-r12
    ldmia sp, {{r0-r12}}
    
    // Deallocate stack space
    add sp, sp, #68
.endm

.macro INVALID_EXCP, kind
.align 2
    SAVE_REGS
    mov r0, sp
    mov r1, #\kind
    bl invalid_exception
    b .Lexception_return
.endm

.macro HANDLE_SYNC
.align 2
    SAVE_REGS
    mov r0, sp
    bl handle_sync_exception
    b .Lexception_return
.endm

.macro HANDLE_IRQ
.align 2
    SAVE_REGS
    mov r0, sp
    bl handle_irq_exception
    b .Lexception_return
.endm

.section .text
.align 5
.global exception_vector_base
exception_vector_base:
    // Reset
    INVALID_EXCP 0
    
    // Undefined Instruction
    INVALID_EXCP 1
    
    // Supervisor Call (SVC)
    HANDLE_SYNC
    
    // Prefetch Abort
    HANDLE_SYNC
    
    // Data Abort
    HANDLE_SYNC
    
    // Reserved
    INVALID_EXCP 5
    
    // IRQ
    HANDLE_IRQ
    
    // FIQ
    INVALID_EXCP 7

.Lexception_return:
    RESTORE_REGS
    movs pc, lr

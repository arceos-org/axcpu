/*
 * ARM32 Exception Vector Table and Trap Handlers
 *
 * ARM32 has 7 exception types at fixed offsets from the vector base:
 *   0x00: Reset
 *   0x04: Undefined Instruction
 *   0x08: Software Interrupt (SVC)
 *   0x0C: Prefetch Abort
 *   0x10: Data Abort
 *   0x14: Reserved
 *   0x18: IRQ
 *   0x1C: FIQ
 *
 * TrapFrame layout (17 words = 68 bytes):
 *   0x00-0x30: r0-r12 (13 registers)
 *   0x34: sp (user/prev mode)
 *   0x38: lr (user/prev mode)
 *   0x3C: pc (return address)
 *   0x40: cpsr (saved program status)
 */

.equ TRAPFRAME_SIZE, 68
.equ TRAPFRAME_R0,   0
.equ TRAPFRAME_SP,   52
.equ TRAPFRAME_LR,   56
.equ TRAPFRAME_PC,   60
.equ TRAPFRAME_CPSR, 64

/*
 * SAVE_REGS_COMMON: Save registers to the SVC mode stack
 *
 * On entry:
 *   - We are in SVC mode (switched from exception mode)
 *   - lr contains the adjusted return address
 *   - r0 contains the saved SPSR
 *   - r1, r2 are scratch
 *
 * Uses the SVC stack for the trap frame.
 */
.macro SAVE_REGS_COMMON
    // Allocate trap frame on SVC stack
    sub sp, sp, #TRAPFRAME_SIZE

    // Save r0-r12 (we'll fix r0-r2 later)
    stmia sp, {{r0-r12}}

    // Save the adjusted return address as PC
    str lr, [sp, #TRAPFRAME_PC]

    // Save SPSR as CPSR
    str r0, [sp, #TRAPFRAME_CPSR]

    // Save dummy sp and lr for now (kernel mode, no user context)
    // For kernel exceptions, save current sp+TRAPFRAME_SIZE and lr
    add r1, sp, #TRAPFRAME_SIZE
    str r1, [sp, #TRAPFRAME_SP]
    mov r1, #0
    str r1, [sp, #TRAPFRAME_LR]
.endm

/*
 * RESTORE_REGS: Restore context and return from exception
 *
 * On entry:
 *   - sp points to TrapFrame
 *   - We are in SVC mode
 */
.macro RESTORE_REGS
    // Disable IRQs during restore to prevent nesting issues
    cpsid i

    // Load return address and saved CPSR
    ldr lr, [sp, #TRAPFRAME_PC]
    ldr r0, [sp, #TRAPFRAME_CPSR]

    // Restore r1-r12 first
    add r1, sp, #4
    ldmia r1, {{r1-r12}}

    // Deallocate trap frame
    add sp, sp, #TRAPFRAME_SIZE

    // Restore SPSR from saved CPSR and r0
    msr spsr_cxsf, r0
    ldr r0, [sp, #-TRAPFRAME_SIZE]  // Restore r0

    // Return from exception
    movs pc, lr
.endm

/*
 * Exception Vector Table
 *
 * Each entry must be exactly 4 bytes (one instruction).
 * We use LDR pc, [pc, #offset] to load the handler address from a table.
 */
.section .text
.align 5
.global exception_vector_base
exception_vector_base:
    ldr pc, _vector_reset           // 0x00: Reset
    ldr pc, _vector_undef           // 0x04: Undefined Instruction
    ldr pc, _vector_svc             // 0x08: SVC (Software Interrupt)
    ldr pc, _vector_prefetch_abort  // 0x0C: Prefetch Abort
    ldr pc, _vector_data_abort      // 0x10: Data Abort
    ldr pc, _vector_reserved        // 0x14: Reserved
    ldr pc, _vector_irq             // 0x18: IRQ
    ldr pc, _vector_fiq             // 0x1C: FIQ

// Handler address table (after the vector table)
.align 2
_vector_reset:          .word _handle_reset
_vector_undef:          .word _handle_undef
_vector_svc:            .word _handle_svc
_vector_prefetch_abort: .word _handle_prefetch_abort
_vector_data_abort:     .word _handle_data_abort
_vector_reserved:       .word _handle_reserved
_vector_irq:            .word _handle_irq
_vector_fiq:            .word _handle_fiq

/*
 * Reset Handler
 * Should not be reached during normal operation.
 */
_handle_reset:
    b .

/*
 * Undefined Instruction Handler
 * LR_und = address of undefined instruction + 4
 */
_handle_undef:
    // Save SPSR and adjust LR
    sub lr, lr, #4              // Point to the undefined instruction
    srsdb sp!, #0x13            // Save LR and SPSR to SVC stack
    cps #0x13                   // Switch to SVC mode

    // Now in SVC mode, save context
    sub sp, sp, #TRAPFRAME_SIZE - 8  // -8 because SRS already pushed 2 words
    stmia sp, {{r0-r12}}

    // Get the saved LR and SPSR from after r12
    add r0, sp, #TRAPFRAME_SIZE - 8
    ldmia r0, {{r1, r2}}        // r1 = LR (PC), r2 = SPSR

    // Store them properly in trap frame
    str r1, [sp, #TRAPFRAME_PC]
    str r2, [sp, #TRAPFRAME_CPSR]

    // Fix stack: remove the SRS-pushed values and adjust sp in frame
    add r0, sp, #TRAPFRAME_SIZE
    str r0, [sp, #TRAPFRAME_SP]
    mov r0, #0
    str r0, [sp, #TRAPFRAME_LR]

    // Call handler
    mov r0, sp
    mov r1, #1                  // UNDEF = 1
    bl invalid_exception
    b .Lexception_return

/*
 * SVC Handler
 * LR_svc = address of SVC instruction + 4
 */
_handle_svc:
    // Already in SVC mode, just save context
    sub lr, lr, #4              // Point to SVC instruction
    mrs r0, spsr                // Get SPSR

    SAVE_REGS_COMMON

    // Restore correct r0 (was clobbered)
    ldr r0, [sp, #TRAPFRAME_R0]
    str r0, [sp, #TRAPFRAME_R0]

    // Call sync exception handler
    mov r0, sp
    bl handle_sync_exception
    b .Lexception_return

/*
 * Prefetch Abort Handler
 * LR_abt = address of aborted instruction + 4
 */
_handle_prefetch_abort:
    sub lr, lr, #4              // Point to the aborted instruction
    srsdb sp!, #0x13            // Save LR and SPSR to SVC stack
    cps #0x13                   // Switch to SVC mode

    sub sp, sp, #TRAPFRAME_SIZE - 8
    stmia sp, {{r0-r12}}

    add r0, sp, #TRAPFRAME_SIZE - 8
    ldmia r0, {{r1, r2}}
    str r1, [sp, #TRAPFRAME_PC]
    str r2, [sp, #TRAPFRAME_CPSR]

    add r0, sp, #TRAPFRAME_SIZE
    str r0, [sp, #TRAPFRAME_SP]
    mov r0, #0
    str r0, [sp, #TRAPFRAME_LR]

    // Call prefetch abort handler
    mov r0, sp
    bl handle_prefetch_abort_exception
    b .Lexception_return

/*
 * Data Abort Handler
 * LR_abt = address of aborted instruction + 8
 */
_handle_data_abort:
    sub lr, lr, #8              // Point to the aborted instruction
    srsdb sp!, #0x13            // Save LR and SPSR to SVC stack
    cps #0x13                   // Switch to SVC mode

    sub sp, sp, #TRAPFRAME_SIZE - 8
    stmia sp, {{r0-r12}}

    add r0, sp, #TRAPFRAME_SIZE - 8
    ldmia r0, {{r1, r2}}
    str r1, [sp, #TRAPFRAME_PC]
    str r2, [sp, #TRAPFRAME_CPSR]

    add r0, sp, #TRAPFRAME_SIZE
    str r0, [sp, #TRAPFRAME_SP]
    mov r0, #0
    str r0, [sp, #TRAPFRAME_LR]

    // Call data abort handler
    mov r0, sp
    bl handle_data_abort_exception
    b .Lexception_return

/*
 * Reserved - should never happen
 */
_handle_reserved:
    b .

/*
 * IRQ Handler
 * LR_irq = address of next instruction to execute + 4
 */
_handle_irq:
    sub lr, lr, #4              // Point to the return address
    srsdb sp!, #0x13            // Save LR and SPSR to SVC stack
    cps #0x13                   // Switch to SVC mode

    sub sp, sp, #TRAPFRAME_SIZE - 8
    stmia sp, {{r0-r12}}

    add r0, sp, #TRAPFRAME_SIZE - 8
    ldmia r0, {{r1, r2}}
    str r1, [sp, #TRAPFRAME_PC]
    str r2, [sp, #TRAPFRAME_CPSR]

    add r0, sp, #TRAPFRAME_SIZE
    str r0, [sp, #TRAPFRAME_SP]
    mov r0, #0
    str r0, [sp, #TRAPFRAME_LR]

    // Call IRQ handler
    mov r0, sp
    bl handle_irq_exception
    b .Lexception_return

/*
 * FIQ Handler
 * LR_fiq = address of next instruction to execute + 4
 */
_handle_fiq:
    sub lr, lr, #4
    srsdb sp!, #0x13
    cps #0x13

    sub sp, sp, #TRAPFRAME_SIZE - 8
    stmia sp, {{r0-r12}}

    add r0, sp, #TRAPFRAME_SIZE - 8
    ldmia r0, {{r1, r2}}
    str r1, [sp, #TRAPFRAME_PC]
    str r2, [sp, #TRAPFRAME_CPSR]

    add r0, sp, #TRAPFRAME_SIZE
    str r0, [sp, #TRAPFRAME_SP]
    mov r0, #0
    str r0, [sp, #TRAPFRAME_LR]

    mov r0, sp
    mov r1, #7                  // FIQ = 7
    bl invalid_exception
    b .Lexception_return

/*
 * Common exception return path
 */
.Lexception_return:
    RESTORE_REGS

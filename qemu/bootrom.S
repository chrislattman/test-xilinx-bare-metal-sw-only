/*
 * bootrom.S: minimal stand-in for the Zynq-7000 BootROM + FSBL when running in
 * upstream QEMU (machine xilinx-zynq-a9). Both CPUs start here (see Makefile).
 *
 * A9#0: enables UART0 (left disabled at reset; on hardware the FSBL/ps7_init
 *       does this) and jumps to hello_world.
 * A9#1: waits like the real BootROM does: WFE until the word at 0xFFFFFFF0 is
 *       non-zero, then jumps there. hello_world writes 0x10000000 to it and
 *       executes SEV, which starts hello_world_1.
 *
 * Linked at 0xFFFFFE00, the top of OCM where the real BootROM keeps A9#1
 * parked and which the hello_world linker script leaves unused.
 * APP_ENTRY (hello_world's ELF entry point) is passed in by the Makefile.
 */
    .syntax unified
    .arm
    .text
    .global _start
_start:
    mrc     p15, 0, r0, c0, c0, 5       // MPIDR
    ands    r0, r0, #3                  // CPU ID
    bne     cpu1_wait

    // A9#0: UART0 8N1, then enable TX/RX (same as XUartPs_EnableUart)
    ldr     r1, =0xE0000000             // UART0 base
    mov     r0, #0x20                   // MR: no parity, 8 data bits, 1 stop bit
    str     r0, [r1, #0x04]
    ldr     r0, =0x114                  // CR: TXEN | RXEN | STPBRK
    str     r0, [r1, #0x00]
    ldr     r0, =APP_ENTRY
    bx      r0

cpu1_wait:
    ldr     r1, =0xFFFFFFF0
1:  wfe
    ldr     r0, [r1]
    cmp     r0, #0
    beq     1b
    bx      r0

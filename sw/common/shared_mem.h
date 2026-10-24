/*
 * shared_mem.h: memory map and handshake shared by A9#0 (hello_world) and A9#1 (hello_world_1)
 *
 * DDR layout (MMU sections are 1 MB, so each region is 1 MB aligned):
 *   0x00100000 - 0x0FEFFFFF  A9#0 private (hello_world lscript.ld)
 *   0x0FF00000 - 0x0FFFFFFF  shared between both cores (this struct)
 *   0x10000000 - 0x1FFFFFFF  A9#1 private (hello_world_1 lscript.ld)
 *
 * DDR is mapped Normal, write-back, Shareable on both cores and both have the
 * ACTLR SMP bit set, so the SCU keeps the L1 caches coherent. No cache
 * flush/invalidate is needed between cores, only ordering (release/acquire).
 */

#ifndef SHARED_MEM_H
#define SHARED_MEM_H

#include <stdatomic.h>
#include <stdint.h>

#include <xil_mmu.h>
#include <xil_types.h>

#define SECTION_SIZE        0x100000U

#define CORE0_PRIVATE_BASE  0x00100000U
#define CORE0_PRIVATE_END   0x0FF00000U
#define SHARED_MEM_BASE     0x0FF00000U
#define CORE1_PRIVATE_BASE  0x10000000U
#define CORE1_PRIVATE_END   0x20000000U

typedef struct {
    // Written by A9#0, read by A9#1 once ready is set
    uint32_t data;
    // 0 = not ready, 1 = data published by A9#0
    atomic_uint ready;
    // 0 = not yet, 1 = data consumed by A9#1
    atomic_uint ack;
} shared_t;

#define SHARED ((shared_t *)SHARED_MEM_BASE)

/*
 * Marks [base, end) as invalid in this core's MMU table, so any access from
 * this core raises a data abort. Invalid (fault) descriptors are honored even
 * though boot.S puts every domain in manager mode, which ignores AP bits.
 */
static inline void unmap_sections(UINTPTR base, UINTPTR end)
{
    for (UINTPTR addr = base; addr < end; addr += SECTION_SIZE) {
        Xil_SetTlbAttributes(addr, 0);
    }
}

#endif

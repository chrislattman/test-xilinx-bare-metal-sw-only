/******************************************************************************
* Copyright (C) 2023 Advanced Micro Devices, Inc. All Rights Reserved.
* SPDX-License-Identifier: MIT
******************************************************************************/
/*
 * helloworld1.c: simple test application
 *
 * This application configures UART 16550 to baud rate 9600.
 * PS7 UART (Zynq) is not initialized by this application, since
 * bootrom/bsp configures it to baud rate 115200
 *
 * ------------------------------------------------
 * | UART TYPE   BAUD RATE                        |
 * ------------------------------------------------
 *   uartns550   9600
 *   uartlite    Configurable only in HW design
 *   ps7_uart    115200 (configured by bootrom/bsp)
 */
 
#include <sleep.h>
#include <stdio.h>
#include "platform.h"
#include "shared_mem.h"
#include "xil_printf.h"

int main()
{
    int counter = 0;
    init_platform();

    // Fault on any access from this core to A9#0's private DDR region (the shared section stays mapped)
    unmap_sections(CORE0_PRIVATE_BASE, CORE0_PRIVATE_END);

    // Wait for A9#0 to publish, then read the data. The acquire load pairs with A9#0's release store,
    // so the data read below cannot be satisfied before the flag is seen.
    while (atomic_load_explicit(&SHARED->ready, memory_order_acquire) == 0)
        ;
    if (SHARED->data == 0xdeadbeef) {
        print("A9#1 read shared data from A9#0\n\r");
        atomic_store_explicit(&SHARED->ack, 1, memory_order_release);
    } else {
        print("A9#1 read unexpected shared data\n\r");
    }

    while (1) {
        sleep(3);
        print("Hello from A9#1!\n\r");
        ++counter;
    }

    cleanup_platform();
    return 0;
}

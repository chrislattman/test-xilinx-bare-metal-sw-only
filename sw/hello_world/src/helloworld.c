/******************************************************************************
* Copyright (C) 2023 Advanced Micro Devices, Inc. All Rights Reserved.
* SPDX-License-Identifier: MIT
******************************************************************************/
/*
 * helloworld.c: simple test application
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
#include <stdint.h>
#include <stdio.h>
#include <xgpio.h>
#include <xil_cache.h>
#include <xinterrupt_wrap.h>
#include <xparameters.h>
#include <xqspips.h>
#include <xstatus.h>

#include "platform.h"

/*
 * This is from the AXI GPIO IP axi_gpio_0 in the block design. The first 
 * channel GPIO is mapped to leds_4bits and the second channel GPIO2 is mapped
 * to rgb_led.
 */
#define LED_CHANNEL 1

/*
 * axi_gpio_1 from the block design has 2 channels. First is mapped to btns_4bits
 * and second is mapped to sws_2bits.
 */
#define BTNS_CHANNEL 1

static XGpio gpio_instance0, gpio_instance1;
static XQspiPs qspi_ps_instance;

static void button_isr()
{
    XGpio_InterruptClear(&gpio_instance1, BTNS_CHANNEL);
    XGpio_DiscreteWrite(&gpio_instance0, LED_CHANNEL, 0x2);
}

int main()
{
    int status;
    XGpio_Config *gpio_config_ptr;
    XQspiPs_Config *qspi_ps_config_ptr;

    // Initialize spinlock to 0 (locked)
    // *(volatile uint32_t *)(XPAR_PS7_DDR_0_BASEADDRESS + 4) = 0;
    // Xil_DCacheFlushRange(XPAR_PS7_DDR_0_BASEADDRESS + 4, 4);

    // Initialize AXI GPIO for LEDs
    status = XGpio_Initialize(&gpio_instance0, XPAR_AXI_GPIO_0_BASEADDR);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    XGpio_SetDataDirection(&gpio_instance0, LED_CHANNEL, 0x0);

    // Using the SDT approach adopted by Vitis Unified IDE
    gpio_config_ptr = XGpio_LookupConfig(XPAR_AXI_GPIO_1_BASEADDR);
    if (!gpio_config_ptr) {
        return XST_FAILURE;
    }
    status = XGpio_Initialize(&gpio_instance1, XPAR_AXI_GPIO_1_BASEADDR);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    status = XSetupInterruptSystem(&gpio_instance1, &button_isr, gpio_config_ptr->IntrId, gpio_config_ptr->IntrParent, XINTERRUPT_DEFAULT_PRIORITY);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    XGpio_InterruptEnable(&gpio_instance1, BTNS_CHANNEL);
    XGpio_InterruptGlobalEnable(&gpio_instance1);

    // Example: writing to and reading from DDR memory (useful for sharing data between multiple cores)
    // Note: Cortex A9 L1 cache line size is 32 bytes
    // Core 0:

    // Write to shared memory:
    volatile uint32_t *shared = (uint32_t *)XPAR_PS7_DDR_0_BASEADDRESS;
    *shared = 0xdeadbeef;
    // Xil_DCacheFlushRange(XPAR_PS7_DDR_0_BASEADDRESS, 4);

    // Note: could use:
    // typedef struct {
    //     uint8_t data[512];
    //     volatile uint8_t flag;
    // } shared_t __attribute__((aligned(32))); // 32 byte alignment to avoid false sharing with cache lines
    // volatile shared_t *shared = (shared_t *)XPAR_PS7_DDR_0_BASEADDRESS;

    // Mark data as written by unlocking spinlock separately:
    // *(volatile uint32_t *)(XPAR_PS7_DDR_0_BASEADDRESS + 4) = 1;
    // Xil_DCacheFlushRange(XPAR_PS7_DDR_0_BASEADDRESS + 4, 4);

    // Core 1:

    // Wait for spinlock to be unlocked:
    // while (*(volatile uint32_t *)(XPAR_PS7_DDR_0_BASEADDRESS + 4) == 0);

    // Read from shared memory:
    // Xil_DCacheInvalidateRange(XPAR_PS7_DDR_0_BASEADDRESS, 4);
    uint32_t retrieved_val = *shared;
    if (retrieved_val != 0xdeadbeef) {
        return XST_FAILURE;
    }

    // Mark data as read by locking spinlock:
    // *(volatile uint32_t *)(XPAR_PS7_DDR_0_BASEADDRESS + 4) = 0;
    // Xil_DCacheFlushRange(XPAR_PS7_DDR_0_BASEADDRESS + 4, 4);

    // Initialize QSPI device
    qspi_ps_config_ptr = XQspiPs_LookupConfig(XPAR_QSPI_BASEADDR);
    if (!qspi_ps_config_ptr) {
        return XST_FAILURE;
    }
    status = XQspiPs_CfgInitialize(&qspi_ps_instance, qspi_ps_config_ptr, qspi_ps_config_ptr->BaseAddress);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    status = XQspiPs_SelfTest(&qspi_ps_instance);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    XQspiPs_SetClkPrescaler(&qspi_ps_instance, XQSPIPS_CLK_PRESCALE_8);
    // Only specify XQSPIPS_MANUAL_START_OPTION at first read/write
    XQspiPs_SetOptions(&qspi_ps_instance, XQSPIPS_FORCE_SSELECT_OPTION | XQSPIPS_MANUAL_START_OPTION | XQSPIPS_HOLD_B_DRIVE_OPTION);
    XQspiPs_SetSlaveSelect(&qspi_ps_instance); // only for first read/write
    // TODO: FlashReadID() and FlashQuadEnable() both done only for first read/write

    // TODO: erase, write, change options, read, then compare result
    // https://github.com/Xilinx/embeddedsw/blob/master/XilinxProcessorIPLib/drivers/qspips/examples/xqspips_flash_lqspi_example.c

    init_platform();
    while (1) {
        // TODO: add watchdog timer
        // https://github.com/Xilinx/embeddedsw/blob/master/XilinxProcessorIPLib/drivers/scuwdt/examples/xscuwdt_polled_example.c
        XGpio_DiscreteWrite(&gpio_instance0, LED_CHANNEL, 0x1); // 0x1 is the bitmask for LED0; 0x2, 0x4, 0x8 for other LEDs
        sleep(1);
        printf("Hello\r\n");
        XGpio_DiscreteWrite(&gpio_instance0, LED_CHANNEL, 0x0); // 0x0 means turn off all LEDs
        sleep(1);
        printf("World!\r\n");
    }
    cleanup_platform();
    return 0;
}

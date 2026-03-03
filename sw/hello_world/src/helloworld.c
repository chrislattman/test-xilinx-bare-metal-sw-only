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
#include <xgpio.h>
#include <xil_exception.h>
#include <xil_printf.h>
#include <xinterrupt_wrap.h>
#include <xparameters.h>

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

XGpio gpio0, gpio1;

void button_isr()
{
    XGpio_InterruptClear(&gpio1, BTNS_CHANNEL);
    XGpio_DiscreteWrite(&gpio0, LED_CHANNEL, 0x2);
}

int main()
{
    int status;
    XGpio_Config *config_ptr;

    // Initialize AXI GPIO for each IP
    status = XGpio_Initialize(&gpio0, XPAR_AXI_GPIO_0_BASEADDR);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    XGpio_SetDataDirection(&gpio0, LED_CHANNEL, 0x0);

    // Using the SDT approach adopted by Vitis Unified IDE
    config_ptr = XGpio_LookupConfig(XPAR_AXI_GPIO_1_BASEADDR);
    status = XGpio_Initialize(&gpio1, XPAR_AXI_GPIO_1_BASEADDR);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    status = XSetupInterruptSystem(&gpio1, &button_isr, config_ptr->IntrId, config_ptr->IntrParent, XINTERRUPT_DEFAULT_PRIORITY);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    XGpio_InterruptEnable(&gpio1, BTNS_CHANNEL);
    XGpio_InterruptGlobalEnable(&gpio1);

    init_platform();
    while (1) {
        XGpio_DiscreteWrite(&gpio0, LED_CHANNEL, 0x1); // 0x1 is the bitmask for LED0; 0x2, 0x4, 0x8 for other LEDs
        sleep(1);
        print("Hello\r\n");
        XGpio_DiscreteWrite(&gpio0, LED_CHANNEL, 0x0); // 0x0 means turn off all LEDs
        sleep(1);
        print("World!\r\n");
    }
    cleanup_platform();
    return 0;
}

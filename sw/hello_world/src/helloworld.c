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
#include <xil_printf.h>
#include <xparameters.h>

#include "platform.h"

/*
 * This is from the AXI GPIO IP axi_gpio_0 in the block design. The first 
 * channel GPIO is mapped to leds_4bits and the second channel GPIO2 is mapped
 * to rgb_led.
 */
#define LED_CHANNEL 1 

int main()
{
    XGpio gpio;
    int status;

    status = XGpio_Initialize(&gpio, XPAR_AXI_GPIO_0_BASEADDR);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }

    XGpio_SetDataDirection(&gpio, LED_CHANNEL, 0x0);

    init_platform();
    while (1) {
        XGpio_DiscreteWrite(&gpio, LED_CHANNEL, 0x1); // 0x1 is the bitmask for LED0; 0x2, 0x4, 0x8 for other LEDs
        sleep(1);
        print("Hello\r\n");
        XGpio_DiscreteWrite(&gpio, LED_CHANNEL, 0x0); // 0x0 mean turn off all LEDs
        sleep(1);
        print("World!\r\n");
    }
    cleanup_platform();
    return 0;
}

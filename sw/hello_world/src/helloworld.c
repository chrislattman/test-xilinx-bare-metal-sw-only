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

#include <ff.h>
#include <sleep.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <xgpio.h>
#include <xil_cache.h>
#include <xil_types.h>
#include <xinterrupt_wrap.h>
#include <xparameters.h>
#include <xqspips.h>
#include <xscutimer.h>
#include <xsdps.h>
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

// Constants for flash read/write commands
#define READ_ID             0x9F
#define READ_ID_RESP        0x9D
#define READ_STATUS_CMD     0x05
#define WRITE_ENABLE_CMD    0x06
#define WRITE_STATUS_CMD    0x01
#define SEC_ERASE_CMD       0xD8
#define QUAD_ENABLED        0x40
#define WRITE_CMD           0x02
#define TEST_ADDRESS        0x055000

// Constants for reading/writing data to flash
#define PAGE_SIZE   256
#define OVERHEAD_SIZE 4

static XGpio gpio_instance0, gpio_instance1;

static void button_isr()
{
    XGpio_InterruptClear(&gpio_instance1, BTNS_CHANNEL);
    XGpio_DiscreteWrite(&gpio_instance0, LED_CHANNEL, 0x2);
}

static int read_write_qspi()
{
    XQspiPs_Config *qspi_ps_config_ptr;
    XQspiPs qspi_ps_instance;
    int status;
    uint8_t write_buffer[OVERHEAD_SIZE + PAGE_SIZE];
    uint8_t read_buffer[OVERHEAD_SIZE + PAGE_SIZE];
    uint8_t read_status_cmd[] = {READ_STATUS_CMD, 0};
    uint8_t write_enable_cmd[] = {WRITE_ENABLE_CMD};
    uint8_t quad_enable_cmd[] = {WRITE_STATUS_CMD, 0};
    uint8_t flash_status[2];

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
    status = XQspiPs_SetClkPrescaler(&qspi_ps_instance, XQSPIPS_CLK_PRESCALE_8);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }

    // Only specify XQSPIPS_MANUAL_START_OPTION at first read/write
    status = XQspiPs_SetOptions(&qspi_ps_instance, XQSPIPS_FORCE_SSELECT_OPTION | XQSPIPS_MANUAL_START_OPTION | XQSPIPS_HOLD_B_DRIVE_OPTION);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }

    // More QSPI flash configuration...
    status = XQspiPs_SetSlaveSelect(&qspi_ps_instance);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    write_buffer[0] = READ_ID;
    write_buffer[1] = 0x23; // 3 dummy bytes
    write_buffer[2] = 0x8;
    write_buffer[3] = 0x9;
    status = XQspiPs_PolledTransfer(&qspi_ps_instance, write_buffer, read_buffer, OVERHEAD_SIZE);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }

    // Enable quad mode
    if (read_buffer[1] == READ_ID_RESP) {
        status = XQspiPs_PolledTransfer(&qspi_ps_instance, read_status_cmd, flash_status, sizeof(read_status_cmd));
        if (status != XST_SUCCESS) {
            return XST_FAILURE;
        }
        quad_enable_cmd[1] = flash_status[1] | 1 << 6;
        status = XQspiPs_PolledTransfer(&qspi_ps_instance, write_enable_cmd, NULL, sizeof(write_enable_cmd));
        if (status != XST_SUCCESS) {
            return XST_FAILURE;
        }
        status = XQspiPs_PolledTransfer(&qspi_ps_instance, quad_enable_cmd, NULL, sizeof(quad_enable_cmd));
        if (status != XST_SUCCESS) {
            return XST_FAILURE;
        }
        while (1) {
            status = XQspiPs_PolledTransfer(&qspi_ps_instance, read_status_cmd, flash_status, sizeof(read_status_cmd));
            if (status != XST_SUCCESS) {
                return XST_FAILURE;
            }
            if (flash_status[0] == QUAD_ENABLED && flash_status[1] == QUAD_ENABLED) {
                break;
            }
        }
    }

    // Erase part of flash
    // This erases one full sector (0x10000 bytes) starting at TEST_ADDRESS
    status = XQspiPs_PolledTransfer(&qspi_ps_instance, write_enable_cmd, NULL, sizeof(write_enable_cmd));
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    write_buffer[0] = SEC_ERASE_CMD;
    write_buffer[1] = (uint8_t)((TEST_ADDRESS & 0xFF0000) >> 16);
    write_buffer[2] = (uint8_t)((TEST_ADDRESS & 0xFF00) >> 8);
    write_buffer[3] = (uint8_t)(TEST_ADDRESS & 0xFF);
    status = XQspiPs_PolledTransfer(&qspi_ps_instance, write_buffer, NULL, OVERHEAD_SIZE);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    while (1) {
        status = XQspiPs_PolledTransfer(&qspi_ps_instance, read_status_cmd, flash_status, sizeof(read_status_cmd));
        if (status != XST_SUCCESS) {
            return XST_FAILURE;
        }
        flash_status[1] |= flash_status[0];
        if ((flash_status[1] & 0x01) == 0) {
            break;
        }
    }

    // Write to same part of flash
    status = XQspiPs_PolledTransfer(&qspi_ps_instance, write_enable_cmd, NULL, sizeof(write_enable_cmd));
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    write_buffer[0] = WRITE_CMD;
    write_buffer[1] = (uint8_t)((TEST_ADDRESS & 0xFF0000) >> 16);
    write_buffer[2] = (uint8_t)((TEST_ADDRESS & 0xFF00) >> 8);
    write_buffer[3] = (uint8_t)(TEST_ADDRESS & 0xFF);
    for (size_t i = 4; i < sizeof(write_buffer); i++) {
        // Fill write buffer with data
        write_buffer[i] = (uint8_t)rand();
    }
    status = XQspiPs_PolledTransfer(&qspi_ps_instance, write_buffer, NULL, OVERHEAD_SIZE + PAGE_SIZE);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    while (1) {
        status = XQspiPs_PolledTransfer(&qspi_ps_instance, read_status_cmd, flash_status, sizeof(read_status_cmd));
        if (status != XST_SUCCESS) {
            return XST_FAILURE;
        }
        flash_status[1] |= flash_status[0];
        if ((flash_status[1] & 0x01) == 0) {
            break;
        }
    }

    // Read same part from flash
    status = XQspiPs_SetOptions(&qspi_ps_instance, XQSPIPS_LQSPI_MODE_OPTION | XQSPIPS_HOLD_B_DRIVE_OPTION);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    status = XQspiPs_LqspiRead(&qspi_ps_instance, read_buffer, TEST_ADDRESS, PAGE_SIZE);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    for (size_t i = 0; i < sizeof(read_buffer) - OVERHEAD_SIZE; i++) {
        if (read_buffer[i] != write_buffer[i + OVERHEAD_SIZE]) {
            return XST_FAILURE;
        }
    }

    return XST_SUCCESS;
}

static int read_write_sd()
{
    FATFS fatfs;
    FRESULT res;
    MKFS_PARM mkfs_parm;
    FIL fil;
    unsigned int bytes_written, bytes_read;
    unsigned char work[FF_MAX_SS];
    const char *path = "0:/";
    const char *filename = "test.txt";
    const char contents[] = "Hello this is the content of test.txt!";
    char read_contents[sizeof(contents)];

    // Mount the file system and allocate the space for it
    res = f_mount(&fatfs, path, 0);
    if (res != FR_OK) {
        return XST_FAILURE;
    }
    mkfs_parm.fmt = FM_FAT32;
    res = f_mkfs(path, &mkfs_parm, work, sizeof(work));
    if (res != FR_OK) {
        return XST_FAILURE;
    }

    // Open file, write to it, read back file, confirm expected bytes, then close file
    // By default, supports 12 character max filenames
    res = f_open(&fil, filename, FA_CREATE_ALWAYS | FA_WRITE | FA_READ);
    if (res != FR_OK) {
        return XST_FAILURE;
    }
    res = f_write(&fil, contents, sizeof(contents), &bytes_written);
    if (res != FR_OK || bytes_written != sizeof(contents)) {
        return XST_FAILURE;
    }
    res = f_lseek(&fil, 0);
    if (res != FR_OK) {
        return XST_FAILURE;
    }
    res = f_read(&fil, read_contents, sizeof(contents), &bytes_read);
    if (res != FR_OK || bytes_read != sizeof(contents)) {
        return XST_FAILURE;
    }
    if (strncmp(contents, read_contents, sizeof(contents)) != 0) {
        return XST_FAILURE;
    }
    res = f_close(&fil);
    if (res != FR_OK) {
        return XST_FAILURE;
    }

    return XST_SUCCESS;
}

int main()
{
    int status;
    XGpio_Config *gpio_config_ptr;
    XScuTimer_Config *timer_config_ptr;
    XScuTimer timer;
    uint32_t tick_10s;

    // Initialize AXI GPIO for LEDs
    // While this example uses AXI GPIO, one could use PS GPIO instead (XGpioPs_*) and bypass the PL
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

    // TODO: to configure MMU to make the first 1 MB section of DDR memory writable by only this processor, use xil_mmu.h and these resources:
    // https://docs.amd.com/r/en-US/oslib_rm/Xil_SetTlbAttributes?tocId=DTOYCPXYHp~TOljwBRJ_BQ
    // https://developer.arm.com/documentation/ddi0406/c/System-Level-Architecture/Virtual-Memory-System-Architecture--VMSA-/Short-descriptor-translation-table-format/Short-descriptor-translation-table-format-descriptors
    // https://developer.arm.com/documentation/ddi0406/c/System-Level-Architecture/Virtual-Memory-System-Architecture--VMSA-/Memory-access-control/Access-permissions
    // https://developer.arm.com/documentation/ddi0406/c/System-Level-Architecture/Virtual-Memory-System-Architecture--VMSA-/Memory-region-attributes/Short-descriptor-format-memory-region-attributes--without-TEX-remap

    // Example: writing to and reading from DDR memory (useful for sharing data between multiple cores)
    // Note: Cortex A9 L1 cache line size is 32 bytes
    // Core 0:

    // Initialize spinlock to 0 (locked)
    // *(volatile uint32_t *)(XPAR_PS7_DDR_0_BASEADDRESS + 4) = 0;
    // Xil_DCacheFlushRange(XPAR_PS7_DDR_0_BASEADDRESS + 4, 4);

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

    // QSPI read/write:
    // Note: writing too often to QSPI flash is not a good idea
    // status = read_write_qspi();
    // if (status != XST_SUCCESS) {
    //     return XST_FAILURE;
    // }

    // SD flash read/write (requires a microSD card to be inserted):
    status = read_write_sd();
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }

    // Initialize 10 second interval periodic timer
    timer_config_ptr = XScuTimer_LookupConfig(XPAR_SCUTIMER_BASEADDR);
    if (!timer_config_ptr) {
        return XST_FAILURE;
    }
    status = XScuTimer_CfgInitialize(&timer, timer_config_ptr, timer_config_ptr->BaseAddr);
    if (status != XST_SUCCESS) {
        return XST_FAILURE;
    }
    tick_10s = (uint32_t)((XPAR_CPU_CORE_CLOCK_FREQ_HZ / 2UL) * 10UL);
    XScuTimer_LoadTimer(&timer, tick_10s);
    XScuTimer_EnableAutoReload(&timer);
    XScuTimer_Start(&timer);

    init_platform();
    while (1) {
        // The Zynq-7000 offers a watchdog timer, example here:
        // https://github.com/Xilinx/embeddedsw/blob/master/XilinxProcessorIPLib/drivers/scuwdt/examples/xscuwdt_polled_example.c
        if (XScuTimer_IsExpired(&timer)) {
            printf("10 second timer went off\r\n");
            XScuTimer_ClearInterruptStatus(&timer);
        }
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

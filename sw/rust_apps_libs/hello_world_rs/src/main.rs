//! A9#0 application: no_std Rust port of `hello_world/src/helloworld.c`.
//!
//! Wakes A9#1, blinks LED0 on the AXI GPIO, lights LED1 when a button is
//! pressed (GPIO interrupt), runs the system watchdog and a 10 second TTC
//! timer, and publishes data to A9#1 through shared DDR.
//!
//! The PS7 UART is already configured to 115200 baud by the bootrom/BSP, and
//! `init_platform()`/`cleanup_platform()` from the C version's platform.c do
//! nothing on the A9, so they have no equivalent here.
#![no_std]
#![no_main]

use core::arch::asm;
use core::convert::Infallible;
use core::ffi::{CStr, c_int, c_void};
use core::ptr;
use core::sync::atomic::Ordering;

use shared_mem::{CORE1_PRIVATE_BASE, CORE1_PRIVATE_END, MSG_LEN, Shared, sections, shared};
use xil_a9_0_sys::*;

/// axi_gpio_0 in the block design: channel 1 is leds_4bits, channel 2 is rgb_led
const LED_CHANNEL: u32 = 1;
/// axi_gpio_1: channel 1 is btns_4bits, channel 2 is sws_2bits
const BTNS_CHANNEL: u32 = 1;

// Flash read/write commands
const READ_ID: u8 = 0x9F;
const READ_ID_RESP: u8 = 0x9D;
const READ_STATUS_CMD: u8 = 0x05;
const WRITE_ENABLE_CMD: u8 = 0x06;
const WRITE_STATUS_CMD: u8 = 0x01;
const SEC_ERASE_CMD: u8 = 0xD8;
const QUAD_ENABLED: u8 = 0x40;
const WRITE_CMD: u8 = 0x02;
const TEST_ADDRESS: u32 = 0x055000;

// Sizes for reading/writing data to flash
const PAGE_SIZE: usize = 256;
const OVERHEAD_SIZE: usize = 4;

// Shared with the GPIO interrupt handler (C: static XGpio gpio_instance0, gpio_instance1)
static mut GPIO0: XGpio = unsafe { core::mem::zeroed() };
static mut GPIO1: XGpio = unsafe { core::mem::zeroed() };

type Result<T = ()> = core::result::Result<T, ()>;

/// `XST_SUCCESS` -> `Ok`, anything else -> `Err`
fn check(status: c_int) -> Result {
    if status == XST_SUCCESS { Ok(()) } else { Err(()) }
}

/// `FR_OK` -> `Ok`, anything else -> `Err`
fn check_fr(res: FRESULT) -> Result {
    if res == FRESULT::FR_OK { Ok(()) } else { Err(()) }
}

/// Prints over the PS7 UART (the Arty Z7's FT2232HQ USB-UART)
fn puts(s: &CStr) {
    unsafe { print(s.as_ptr()) }
}

/// A9#0 owns UART0, so it also prints the messages A9#1 posts to the mailbox
fn print_core1_messages(shared: &Shared) {
    let mut msg = [0u8; MSG_LEN];
    // SAFETY: A9#0 is the mailbox's only consumer
    while unsafe { shared.pop(&mut msg) }.is_some() {
        if let Ok(s) = CStr::from_bytes_until_nul(&msg) {
            puts(s);
        }
    }
}

extern "C" fn button_isr(_data: *mut c_void) {
    unsafe {
        XGpio_InterruptClear(&raw mut GPIO1, BTNS_CHANNEL);
        XGpio_DiscreteWrite(&raw mut GPIO0, LED_CHANNEL, 0x2);
    }
}

/// `XQspiPs_PolledTransfer()` of all of `send`, receiving into `recv` (which
/// must be at least as long) or discarding the received bytes
fn qspi_transfer(qspi: &mut XQspiPs, send: &mut [u8], recv: Option<&mut [u8]>) -> Result {
    let recv = match recv {
        Some(r) => {
            assert!(r.len() >= send.len());
            r.as_mut_ptr()
        }
        None => ptr::null_mut(),
    };
    check(unsafe { XQspiPs_PolledTransfer(qspi, send.as_mut_ptr(), recv, send.len() as u32) })
}

/// Polls the flash status register until the write/erase in progress bit clears
fn qspi_wait_ready(qspi: &mut XQspiPs) -> Result {
    let mut read_status_cmd = [READ_STATUS_CMD, 0];
    let mut flash_status = [0u8; 2];
    loop {
        qspi_transfer(qspi, &mut read_status_cmd, Some(&mut flash_status))?;
        flash_status[1] |= flash_status[0];
        if flash_status[1] & 0x01 == 0 {
            return Ok(());
        }
    }
}

/// Enables quad mode, then erases, writes and reads back a page of QSPI flash.
/// Note: writing too often to QSPI flash is not a good idea.
#[allow(dead_code)]
fn read_write_qspi() -> Result {
    let mut qspi = XQspiPs::default();
    let mut write_buffer = [0u8; OVERHEAD_SIZE + PAGE_SIZE];
    let mut read_buffer = [0u8; OVERHEAD_SIZE + PAGE_SIZE];
    let mut read_status_cmd = [READ_STATUS_CMD, 0];
    let mut write_enable_cmd = [WRITE_ENABLE_CMD];
    let mut quad_enable_cmd = [WRITE_STATUS_CMD, 0];
    let mut flash_status = [0u8; 2];

    // Initialize QSPI device
    let config = unsafe { XQspiPs_LookupConfig(XPAR_QSPI_BASEADDR as UINTPTR).as_mut() }.ok_or(())?;
    let base = config.BaseAddress;
    unsafe {
        check(XQspiPs_CfgInitialize(&mut qspi, config, base))?;
        check(XQspiPs_SelfTest(&mut qspi))?;
        check(XQspiPs_SetClkPrescaler(&mut qspi, XQSPIPS_CLK_PRESCALE_8 as u8))?;
        // Only specify XQSPIPS_MANUAL_START_OPTION at first read/write
        check(XQspiPs_SetOptions(
            &mut qspi,
            XQSPIPS_FORCE_SSELECT_OPTION | XQSPIPS_MANUAL_START_OPTION | XQSPIPS_HOLD_B_DRIVE_OPTION,
        ))?;
        check(XQspiPs_SetSlaveSelect(&mut qspi))?;
    }

    write_buffer[..OVERHEAD_SIZE].copy_from_slice(&[READ_ID, 0x23, 0x08, 0x09]); // 3 dummy bytes
    qspi_transfer(
        &mut qspi,
        &mut write_buffer[..OVERHEAD_SIZE],
        Some(&mut read_buffer[..OVERHEAD_SIZE]),
    )?;

    // Enable quad mode
    if read_buffer[1] == READ_ID_RESP {
        qspi_transfer(&mut qspi, &mut read_status_cmd, Some(&mut flash_status))?;
        quad_enable_cmd[1] = flash_status[1] | 1 << 6;
        qspi_transfer(&mut qspi, &mut write_enable_cmd, None)?;
        qspi_transfer(&mut qspi, &mut quad_enable_cmd, None)?;
        loop {
            qspi_transfer(&mut qspi, &mut read_status_cmd, Some(&mut flash_status))?;
            if flash_status[0] == QUAD_ENABLED && flash_status[1] == QUAD_ENABLED {
                break;
            }
        }
    }

    let [_, a2, a1, a0] = TEST_ADDRESS.to_be_bytes();

    // Erase part of flash: one full sector (0x10000 bytes) starting at TEST_ADDRESS
    qspi_transfer(&mut qspi, &mut write_enable_cmd, None)?;
    write_buffer[..OVERHEAD_SIZE].copy_from_slice(&[SEC_ERASE_CMD, a2, a1, a0]);
    qspi_transfer(&mut qspi, &mut write_buffer[..OVERHEAD_SIZE], None)?;
    qspi_wait_ready(&mut qspi)?;

    // Write to same part of flash
    qspi_transfer(&mut qspi, &mut write_enable_cmd, None)?;
    write_buffer[..OVERHEAD_SIZE].copy_from_slice(&[WRITE_CMD, a2, a1, a0]);
    for b in &mut write_buffer[OVERHEAD_SIZE..] {
        *b = unsafe { rand() } as u8;
    }
    qspi_transfer(&mut qspi, &mut write_buffer, None)?;
    qspi_wait_ready(&mut qspi)?;

    // Read same part from flash
    unsafe {
        check(XQspiPs_SetOptions(&mut qspi, XQSPIPS_LQSPI_MODE_OPTION | XQSPIPS_HOLD_B_DRIVE_OPTION))?;
        check(XQspiPs_LqspiRead(&mut qspi, read_buffer.as_mut_ptr(), TEST_ADDRESS, PAGE_SIZE as u32))?;
    }
    if read_buffer[..PAGE_SIZE] != write_buffer[OVERHEAD_SIZE..] {
        return Err(());
    }
    Ok(())
}

/// Formats the SD card as FAT32, then writes, reads back and checks a file.
/// Requires a microSD card to be inserted.
#[allow(dead_code)]
fn read_write_sd() -> Result {
    const CONTENTS: &[u8] = b"Hello this is the content of test.txt!\0";
    let path = c"0:/";
    // By default, supports 12 character max filenames
    let filename = c"test.txt";

    let mut fatfs = FATFS::default();
    let mut fil = FIL::default();
    let mkfs_parm = MKFS_PARM { fmt: FM_FAT32 as BYTE, ..Default::default() };
    let mut work = [0u8; FF_MAX_SS as usize];
    let mut read_contents = [0u8; CONTENTS.len()];
    let (mut bytes_written, mut bytes_read): (UINT, UINT) = (0, 0);

    unsafe {
        // Mount the file system and allocate the space for it
        check_fr(f_mount(&mut fatfs, path.as_ptr(), 0))?;
        check_fr(f_mkfs(path.as_ptr(), &mkfs_parm, work.as_mut_ptr().cast(), work.len() as UINT))?;

        // Open file, write to it, read back file, confirm expected bytes, then close file
        check_fr(f_open(&mut fil, filename.as_ptr(), (FA_CREATE_ALWAYS | FA_WRITE | FA_READ) as BYTE))?;
        check_fr(f_write(&mut fil, CONTENTS.as_ptr().cast(), CONTENTS.len() as UINT, &mut bytes_written))?;
        if bytes_written as usize != CONTENTS.len() {
            return Err(());
        }
        check_fr(f_lseek(&mut fil, 0))?;
        check_fr(f_read(&mut fil, read_contents.as_mut_ptr().cast(), CONTENTS.len() as UINT, &mut bytes_read))?;
        if bytes_read as usize != CONTENTS.len() || read_contents != CONTENTS {
            return Err(());
        }
        check_fr(f_close(&mut fil))?;
    }
    Ok(())
}

/// TTC prescaler and interval for a period of `total_ticks` input clock cycles:
/// the smallest prescaler (divide by 2^(n+1)) that fits the interval in the
/// 16-bit counter
fn ttc_divisors(total_ticks: u32) -> (u8, u32) {
    let prescaler = (0u8..16).find(|p| total_ticks >> (p + 1) <= 0xFFFF).unwrap_or(16);
    (prescaler, total_ticks >> (prescaler + 1))
}

fn run() -> Result<Infallible> {
    let shared = shared();

    // Fault on any access from this core to A9#1's private DDR region
    // (the debugger and DMA masters bypass the MMU and are not affected)
    for addr in sections(CORE1_PRIVATE_BASE, CORE1_PRIVATE_END) {
        unsafe { Xil_SetTlbAttributes(addr as INTPTR, 0) };
    }

    // Initialize the handshake flags before waking A9#1, since DDR holds random values at power-up
    shared.ready.store(0, Ordering::Relaxed);
    shared.ack.store(0, Ordering::Relaxed);
    shared.msg_head.store(0, Ordering::Relaxed);
    shared.msg_tail.store(0, Ordering::Relaxed);

    // Zynq 7000-specific code to wake A9#1: write its entry point where the
    // BootROM's WFE loop looks for it, then send an event. The dmb() also makes
    // the flag initialization visible to A9#1 before it starts.
    unsafe {
        Xil_Out32(0xFFFF_FFF0, CORE1_PRIVATE_BASE as u32);
        dmb();
        asm!("sev", options(nomem, nostack, preserves_flags));
    }

    // Initialize AXI GPIO for LEDs
    // While this example uses AXI GPIO, one could use PS GPIO instead (XGpioPs_*) and bypass the PL
    unsafe {
        check(XGpio_Initialize(&raw mut GPIO0, XPAR_XGPIO_0_BASEADDR as UINTPTR))?;
        XGpio_SetDataDirection(&raw mut GPIO0, LED_CHANNEL, 0x0);
    }

    // Using the SDT approach adopted by Vitis Unified IDE
    let gpio_config = unsafe { XGpio_LookupConfig(XPAR_XGPIO_1_BASEADDR as UINTPTR).as_ref() }.ok_or(())?;
    unsafe {
        check(XGpio_Initialize(&raw mut GPIO1, XPAR_XGPIO_1_BASEADDR as UINTPTR))?;
        // Abstracts away setup of the SCU GIC
        check(XSetupInterruptSystem(
            (&raw mut GPIO1).cast(),
            button_isr as *mut c_void,
            gpio_config.IntrId.into(),
            gpio_config.IntrParent,
            XINTERRUPT_DEFAULT_PRIORITY as u16,
        ))?;
        XGpio_InterruptEnable(&raw mut GPIO1, BTNS_CHANNEL);
        XGpio_InterruptGlobalEnable(&raw mut GPIO1);
    }

    // Share data with A9#1 through the shared DDR section. The release store
    // orders the data write before the flag write (emits dmb), pairing with
    // A9#1's acquire load of the flag. For a lock that both cores can take,
    // use compare_exchange (ldrex/strex) instead.
    shared.data.store(0xdead_beef, Ordering::Relaxed);
    shared.ready.store(1, Ordering::Release);
    let mut ack_printed = false;

    // QSPI read/write:
    // read_write_qspi()?;

    // SD flash read/write (requires a microSD card to be inserted):
    // read_write_sd()?;

    // When debugging, comment out the watchdog (and the sleep() calls on both cores)

    // Initialize 4 second system watchdog timer
    let mut wdt = XWdtPs::default();
    let wdt_config = unsafe { XWdtPs_LookupConfig(XPAR_XWDTPS_0_BASEADDR).as_mut() }.ok_or(())?;
    let base = wdt_config.BaseAddress;
    unsafe {
        check(XWdtPs_CfgInitialize(&mut wdt, wdt_config, base))?;
        XWdtPs_SetControlValue(&mut wdt, XWDTPS_COUNTER_RESET as u8, 0xFFFF);
        XWdtPs_SetControlValue(&mut wdt, XWDTPS_CLK_PRESCALE as u8, XWDTPS_CCR_PSCALE_4096);
        XWdtPs_EnableOutput(&mut wdt, XWDTPS_RESET_SIGNAL as u8);
        XWdtPs_Start(&mut wdt);
    }

    // Initialize 10 second interval periodic timer
    let mut ttc = XTtcPs::default();
    let ttc_config = unsafe { XTtcPs_LookupConfig(XPAR_XTTCPS_0_BASEADDR).as_mut() }.ok_or(())?;
    let (base, input_clock_hz) = (ttc_config.BaseAddress, ttc_config.InputClockHz);
    let (prescaler, interval) = ttc_divisors(input_clock_hz * 10);
    unsafe {
        check(XTtcPs_CfgInitialize(&mut ttc, ttc_config, base))?;
        XTtcPs_SetOptions(&mut ttc, XTTCPS_OPTION_INTERVAL_MODE | XTTCPS_OPTION_WAVE_DISABLE);
        XTtcPs_SetPrescaler(&mut ttc, prescaler);
        XTtcPs_SetInterval(&mut ttc, interval as XInterval);
        XTtcPs_ResetCounterValue(&mut ttc);
        XTtcPs_EnableInterrupts(&mut ttc, XTTCPS_IXR_INTERVAL_MASK);
        XTtcPs_Start(&mut ttc);
    }

    loop {
        unsafe {
            if XTtcPs_GetInterruptStatus(&mut ttc) & XTTCPS_IXR_INTERVAL_MASK != 0 {
                puts(c"10 second timer went off\r\n");
                XTtcPs_ClearInterruptStatus(&mut ttc, XTTCPS_IXR_INTERVAL_MASK);
            }
            if !ack_printed && shared.ack.load(Ordering::Acquire) != 0 {
                puts(c"A9#1 received shared data\r\n");
                ack_printed = true;
            }
            // 0x1 is the bitmask for LED0; 0x2, 0x4, 0x8 for the other LEDs
            XGpio_DiscreteWrite(&raw mut GPIO0, LED_CHANNEL, 0x1);
            sleep(1);
            print_core1_messages(shared);
            puts(c"Hello\r\n");
            // 0x0 turns off all LEDs
            XGpio_DiscreteWrite(&raw mut GPIO0, LED_CHANNEL, 0x0);
            sleep(1);
            print_core1_messages(shared);
            puts(c"World!\r\n");
            XWdtPs_RestartWdt(&mut wdt);
        }
    }
}

/// Called by the BSP's startup code (xil-crt0.S) after it sets up the MMU,
/// caches, stacks and .bss
#[unsafe(no_mangle)]
extern "C" fn main() -> c_int {
    match run() {
        Ok(never) => match never {},
        Err(()) => XST_FAILURE,
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    puts(c"A9#0 panicked\r\n");
    loop {}
}

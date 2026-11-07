## Bare-metal software applications

This project contains the C bare-metal (standalone) applications used to print messages over UART and toggle an LED on and off, similar to [this](https://github.com/chrislattman/test-bare-metal) example.

Vitis Unified IDE projects are organized around platform components and application components:

- A platform component (`artyz7_platform`) takes in the XSA file from Vivado and extracts it into a board support package (BSP)
- An application component (`hello_world`) is a software application that runs on top of the BSP

A domain is a target OS/processor pair and is part of the platform component. The target OS determines what kind of application components can run on it.
An application component can be a standalone application, a FreeRTOS application, or a PetaLinux application.

Note: before running any `vitis`, `bootgen`, `program_flash`, `hw_server`, or `xsdb` commands *outside* of the Vitis Unified IDE, make sure to source the Vitis environment variables.

To deploy the project using JTAG without the Vitis IDE:

- Run `vitis -s build.py --release`
- Run `hw_server -s tcp:127.0.0.1:3121` in one shell
    - `hw_server -s tcp::3121` to enable remote running
- Run `vitis -s run.py` in another shell

To debug the project from the command line (using TCL script for demonstration, but Python works):

- Run `vitis -s build.py`
- Run `hw_server -s tcp:127.0.0.1:3121` in one shell
    - `hw_server -s tcp::3121` to enable remote debugging
- Run `xsdb` in another shell, then in the xsdb shell, run `source debug.tcl`
    - Commands are available [here](https://docs.amd.com/r/en-US/ug1725-xsdb-reference-guide/XSDB-Commands)

When debugging, there is a built-in serial monitor available in Vitis Unified IDE 2025.1 and later. Go to Vitis -> Serial Monitor, select your device's serial port, then select the 115200 baud rate.

If you want to program QSPI flash so that the device starts executing code from a boot image:

- Run `bootgen -image system.bif -arch zynq -o BOOT.bin -w` from the `hello_world` folder
    - For the Rust applications, build them with `cargo build --release` and use `system_rust.bif` instead
- Run `hw_server -s tcp:127.0.0.1:3121` in one shell
- In another shell, run `program_flash -f hello_world/BOOT.bin -fsbl artyz7_platform/export/artyz7_platform/sw/boot/fsbl.elf -flash_type qspi-x4-single -blank_check -verify -target_name "jsn-Arty Z7-003017BB13CFA-4ba00477-0"` from the `sw` folder
- Unplug power from the board, switch from JTAG to QSPI boot mode on the board, then plug power back in

Notes:

- Don't pin Vitis to the Windows taskbar. It must be run from the Desktop or Start Menu shortcuts since those shortcuts load the settings64.bat file
- If you want to initialize the board with ps7_init.tcl instead of the FSBL, change `isFsbl` to `false` in `hello_world/_ide/launch.json`
- If you want one A9 to communicate to another (in Linux this would be interprocess communication), the best approach is to use shared (DDR) memory and interprocessor interrupts, also known as software generated interrupts
- The cores don't share peripherals, following Xilinx's guidance for AMP (the drivers have no locking, except the GIC's)
    - A9#0 owns UART0. A9#1 never prints: it posts messages to a mailbox in shared DDR (`msg_post()` in `shared_mem/shared_mem.h`, `Shared::post` in Rust), and A9#0 prints them after each `sleep()`, so they can appear up to a second late
    - A9#0 owns the global timer, which both cores share and which `xiltimer`'s `sleep()` resets on first use. A9#1's `sleep()` uses its own SCU private timer instead: `build.py` sets its `XILTIMER_sleep_timer` to `ps7_scutimer_0`
    - In Linux you would use Unix domain sockets
- `hw_server` is needed so that `xsdb` and other utilities can connect to one central JTAG daemon
- When debugging with Vitis, cross-trigger applies to both Run and Debug configurations (keep it off during Runs)
    - `ps7_cortexa9_0` must come first in `downloadElf` in `system_project/_ide/launch.json`, otherwise halting A9#0 to load its ELF fires the cross-trigger and leaves A9#1 halted
    - Continuing A9#0 from a breakpoint steps over it first, which briefly halts A9#0 and fires the cross-trigger, so A9#1 halts again and must be continued
    - Cross-trigger doesn't work as cleanly in VSCode (cppdbg + GDB) as it does in Vitis. With `set schedule-multiple off`, GDB only runs one core at a time, so A9#1 is already halted whenever A9#0 hits a breakpoint and the cross-trigger has nothing to do
        - A9#1 can be resumed outside GDB (e.g. `targets 3; con` in xsdb) so the cross-trigger halts it, but GDB didn't resume A9#1 itself, so its view of A9#1 may be stale (untested)
        - The cross-trigger breakpoint (`bpadd -ct-input 0 -ct-output 8` in `vscode_debug.tcl`) only lasts while the xsdb session that set it stays connected
- A9#1's BSP must be built with `-DUSE_AMP=1`, otherwise its `boot.S` invalidates the SCU and the shared L2 cache when A9#1 starts, wiping A9#0's cached data (e.g. its zeroed `.bss`)
    - In Vitis 2025.2, the BSP's `proc_extra_compiler_flags` setting is not applied to the BSP build, so `-DUSE_AMP=1` is instead added to `TOOLCHAIN_C_FLAGS`, `TOOLCHAIN_CXX_FLAGS` and `TOOLCHAIN_ASM_FLAGS` in `artyz7_platform/ps7_cortexa9_1/standalone_ps7_cortexa9_1/bsp/cortexa9_toolchain.cmake`
    - A clean platform rebuild keeps that edit, but recreating the A9#1 domain loses it
    - After changing it, clean build the platform, then clean build `hello_world_1` (an incremental app build may not relink against the new BSP)

## Rust applications

`sw/` is also a Cargo workspace (target `armv7a-none-eabihf`, set in `.cargo/config.toml`) with `no_std` Rust ports of the two applications, built on bindings to the two non-FSBL standalone BSPs:

- `rust_apps_libs/hello_world_rs`: A9#0 application, port of `hello_world/src/helloworld.c`
- `rust_apps_libs/hello_world_1_rs`: A9#1 application, port of `hello_world_1/src/helloworld1.c`
- `rust_apps_libs/shared_mem`: Rust version of `shared_mem/shared_mem.h` (memory map and handshake; same layout, so a Rust core can talk to a C core)
- `rust_platform/xil_a9_0_sys`, `rust_platform/xil_a9_1_sys`: bindgen bindings to `artyz7_platform/ps7_cortexa9_{0,1}/standalone_ps7_cortexa9_{0,1}/bsp`
- `rust_platform/xil_bsp_build`: host-side `build.rs` helper both sys crates use (bindgen, C shims, link setup)

The C BSPs must be built before running `cargo build`: build the platform (`artyz7_platform`) in Vitis first (`vitis -s build.py --no-apps` builds just the platform, skipping the C applications), since the bindings are generated from each BSP's `include/` and link against its `lib/*.a`. Then source the Vitis environment and run `cargo build` (or `cargo build --release`) from `sw/`. The ELFs are `sw/target/armv7a-none-eabihf/<debug|release>/hello-world-rs` and `hello-world-1-rs`.

To debug them, use the `Rust: ...` configurations in `.vscode/launch.json` (Native Debug extension with `rust-gdb`). On the board, run `xsdb vscode_debug.tcl rust` first, which loads the Rust ELFs instead of the C ones. To run or debug them in QEMU, see the root `README.md`.

Notes:

- Each app has its own `linker.ld`, a copy of its C counterpart's `lscript.ld`. Keep the copies in sync with the C apps' when you change the memory map (it must also match `rust_apps_libs/shared_mem`)
- The whole BSP is exposed: `build.rs` generates a wrapper that includes every header in the BSP's `include/` directory, so every driver, `libxil*` function, struct, enum, typedef, global and constant `#define` is available. Regenerating the platform with more drivers picks them up automatically
    - `SKIP_HEADERS` in `rust_platform/xil_bsp_build/src/lib.rs` lists headers left out because they can't be parsed for this target (`xil_cache_vxworks.h`, `xil_macroback.h`)
    - bindgen drops constant `#define`s it can't evaluate (e.g. ones with casts or function calls)
- `static inline` functions (`Xil_In32`, `Xil_Out32`, ...) are wrapped automatically. Function-like macros can't be, so only the ones in `rust_platform/xil_bsp_build/c/xilrs_shim.c` are available (`XTtcPs_Start`, `XTtcPs_SetInterval`, `XWdtPs_RestartWdt`, `dmb`, ...). Add a `xilrs_<Name>` wrapper there when you need another. Both kinds are called from Rust by their C names
- `XPAR_*` constants are `u32`, but some drivers take `UINTPTR` (`usize`), so cast with `as UINTPTR` where C would convert implicitly
- The BSP archives, `libgcc` and newlib's `libc` link as one group, which requires linking with (Vitis) `arm-none-eabi-gcc`. Each app's `build.rs` passes its BSP's `Xilinx.spec` and linker script

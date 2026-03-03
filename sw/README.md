## Bare-metal software application

This project contains the C bare-metal (standalone) application used to print messages over UART and toggle an LED on and off, similar to [this](https://github.com/chrislattman/test-bare-metal) example.

Vitis Unified IDE projects are organized around platform components and application components:

- A platform component (`artyz7_platform`) takes in the XSA file from Vivado and extracts it into a board support package (BSP)
- An application component (`hello_world`) is a software application that runs on top of the BSP

A domain is a target OS/processor pair and is part of the platform component. The target OS determines what kind of application components can run on it.
An application component can be a standalone application, a FreeRTOS application, or a PetaLinux application.

Note: before running any `vitis`, `bootgen`, `program_flash`, `hw_server`, or `xsdb` commands *outside* of the Vitis Unified IDE, make sure to source the Vitis environment variables.

To deploy the project using JTAG without the Vitis IDE:

- Run `vitis -s build.py --release`
- Run `hw_server -s tcp:127.0.0.1:3121` in one shell
- Run `vitis -s run.py` in another shell

To debug the project from the command line (using TCL script for demonstration, but Python works):

- Run `vitis -s build.py`
- Run `hw_server -s tcp:127.0.0.1:3121` in one shell
- Run `xsdb` in another shell, then in the xsdb shell, run `source debug.tcl`
    - Commands are available [here](https://docs.amd.com/r/en-US/ug1725-xsdb-reference-guide/XSDB-Commands)

When debugging, there is a built-in serial monitor available in Vitis Unified IDE 2025.1 and later. Go to Vitis -> Serial Monitor, select your device's serial port, then select the 115200 baud rate.

If you want to program QSPI flash so that the device starts executing code from a boot image:

- Run `bootgen -image system.bif -arch zynq -o BOOT.bin -w` from the `hello_world` folder
- Run `hw_server -s tcp:127.0.0.1:3121` in one shell
- In another shell, run `program_flash -f hello_world/BOOT.bin -fsbl artyz7_platform/export/artyz7_platform/sw/boot/fsbl.elf -flash_type qspi-x4-single -blank_check -verify -target_name "jsn-Arty Z7-003017BB13CFA-4ba00477-0"` from the `sw` folder
- Unplug power from the board, switch from JTAG to QSPI boot mode on the board, then plug power back in

Notes:

- Don't pin Vitis to the Windows taskbar. It must be run from the Desktop or Start Menu shortcuts since those shortcuts load the settings64.bat file
- If you want to initialize the board with ps7_init.tcl instead of the FSBL, change `isFsbl` to `false` in `hello_world/_ide/launch.json`
- If you want one A9 to communicate to another (in Linux this would be interprocess communication), the best approach is to use shared (DDR) memory and interprocessor interrupts, also known as software generated interrupts
    - In Linux you would use Unix domain sockets
- `hw_server` is needed so that `xsdb` and other utilities can connect to one central JTAG daemon

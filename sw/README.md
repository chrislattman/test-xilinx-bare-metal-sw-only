## Bare-metal software application

This project contains the C bare-metal (standalone) application used to print messages over UART and toggle an LED on and off, similar to [this](https://github.com/chrislattman/test-bare-metal) example.

Vitis Unified IDE projects are organized around platform components and application components:

- A platform component (`artyz7_platform`) takes in the XSA file from Vivado and extracts it into a board support package (BSP)
- An application component (`hello_world`) is a software application that runs on top of the BSP

A domain is a target OS/processor pair and is part of the platform component. The target OS determines what kind of application components can run on it.
An application component can be a standalone application, a FreeRTOS application, or a PetaLinux application.

To build the project non-interactively, run `vitis -s build.py` from the `sw` folder. To generate the BOOT.bin file for QSPI flash or QEMU, run `bootgen -image system.bif -arch zynq -o BOOT.bin -w` from the `hello_world` folder.

For secure boot (testing only):

- Generate the primary and secondary secret keys (PSK and SSK) by running `bootgen -generate_keys pem -arch zynq -image generate_pem.bif` from the `hello_world` folder
    - These are technically RSA public/private key pairs
- Generate the secure boot-enabled BOOT.bin by running `bootgen -image secure.bif -arch zynq -o BOOT.bin -w -encrypt bbram`
    - This makes bootgen generate an AES key (stored in BBRAM)
    - bh_auth_enable in `secure.bif` ensures that eFUSEs aren't written to

When debugging, there is a built-in serial monitor available in Vitis Unified IDE 2025.1 and later. Go to Vitis -> Serial Monitor, your device's serial port, then select the 115200 baud rate.
To debug the application from the command line, use XSDB. It internally uses the same debugging protocol as GDB but XSDB also understands FPGA logic (which GDB does not, of course).

Notes:

- Don't pin Vitis to the Windows taskbar. It must be run from the Desktop or Start Menu shortcuts since those shortcuts load the settings64.bat file
- If you want to initialize the board with ps7_init.tcl instead of the FSBL, change `isFsbl` to `false` in `hello_world/_ide/launch.json`

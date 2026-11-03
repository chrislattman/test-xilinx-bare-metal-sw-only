# Run hello_world (A9#0) and hello_world_1 (A9#1) together in upstream QEMU's
# xilinx-zynq-a9 machine. They have to run together: A9#1 is only released by
# hello_world, and the two exchange data through shared DDR.
#
# Build the applications first (Vitis IDE or `vitis -s sw/build.py`); these
# targets use the existing ELFs and do not rebuild them.
#
# Not emulated: the PL (AXI GPIO writes to 0x41200000/0x41210000 are ignored)
# and the system watchdog, so it never resets the board. Timing is approximate.

QEMU        ?= qemu-system-arm
CROSS       ?= arm-none-eabi-
TELNET_PORT ?= 5678

ELF0    := sw/hello_world/build/hello_world.elf
ELF1    := sw/hello_world_1/build/hello_world_1.elf
BOOTROM := build/qemu/bootrom.elf

# Both cores print to UART0 (first -serial); UART1 is unused.
# hello_world_1 is loaded without cpu-num, so it is only copied into DDR. A9#0
# starts at the boot stub, which jumps to hello_world; A9#1 starts at the
# stub's wait loop (0xFFFFFE00) until hello_world releases it.
QEMU_ARGS := -M xilinx-zynq-a9 -smp 2 -m 512M -nographic \
             -serial telnet:localhost:$(TELNET_PORT),server=on,wait=off \
             -serial null \
             -device loader,file=$(ELF0) \
             -device loader,file=$(ELF1) \
             -device loader,file=$(BOOTROM),cpu-num=0 \
             -device loader,addr=0xFFFFFE00,cpu-num=1

.PHONY: qemu qemu_debug

# In another shell: telnet localhost 5678
# Quit qemu: (qemu) quit
# Quit telnet: Ctrl + ] -> telnet> quit
# Output printed before telnet connects is lost (wait=off); use qemu_debug to
# start halted instead.
qemu: $(BOOTROM)
	$(QEMU) $(QEMU_ARGS)

# Same as qemu, but halts at startup and waits for GDB on localhost:1234
# (replace -s with -gdb tcp:localhost:<PORT> to use a different port).
# Both cores are threads of one inferior (thread 1 = A9#0, thread 2 = A9#1).
# Use the "QEMU A9#0 + A9#1 Debug" configuration in .vscode/launch.json, or:
#   gdb-multiarch -q sw/hello_world/build/hello_world.elf
#   (gdb) add-symbol-file sw/hello_world_1/build/hello_world_1.elf
#   (gdb) target remote localhost:1234
#   (gdb) set scheduler-locking step
#   (gdb) break main
#   (gdb) continue
# In another shell: telnet localhost 5678
qemu_debug: $(BOOTROM)
	$(QEMU) $(QEMU_ARGS) -S -s

# Rebuilt whenever hello_world.elf changes, since its entry point is baked in.
$(BOOTROM): qemu/bootrom.S $(ELF0) $(ELF1)
	@mkdir -p $(dir $@)
	$(CROSS)gcc -mcpu=cortex-a9 -nostdlib -g -Wl,-Ttext=0xFFFFFE00 -Wl,--build-id=none \
		-DAPP_ENTRY=$$($(CROSS)readelf -h $(ELF0) | awk '/Entry point/ {print $$4}') \
		-o $@ $<

$(ELF0) $(ELF1):
	$(error $@ not found; build the applications in Vitis first)

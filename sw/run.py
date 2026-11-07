import os
import time
import xsdb

# The Python API's filter parser is not the Tcl one: it rejects quotes and &&, so
# values go unquoted (multi-word values still work) and each condition needs its own
# parentheses, joined with "and". Characters such as / or . are also rejected; use *.
CABLE = "(jtag_cable_name =~ Digilent Arty Z7 003017BB13CFA)"
PS_DAP = "(jtag_device_ctx == jsn-Arty Z7-003017BB13CFA-4ba00477-0)"
PL = "(jtag_device_ctx == jsn-Arty Z7-003017BB13CFA-23727093-0)"

# hw_server resolves ELF paths against its own working directory when loading symbols,
# so relative paths break breakpoints on symbols like FsblHandoffJtagExit.
SW = os.path.dirname(os.path.abspath(__file__))

session = xsdb.start_debug_session()
# session.connect(host="173.673.3.2", port=3121) if remote running to hw_server at 173.673.3.2:3121
session.connect()
session.bpremove("--all")
session.targets("-sn", filter=f"(name =~ APU*) and {CABLE} and {PS_DAP}")
session.rst()
time.sleep(3)

session.targets("-s", filter=f"{CABLE} and (level == 0) and {PL}")
session.fpga(file=f"{SW}/hello_world/_ide/bitstream/artyz7_wrapper.bit")
session.targets("-sn", filter=f"(name =~ APU*) and {CABLE} and {PS_DAP}")
session.loadhw(hw=f"{SW}/artyz7_platform/export/artyz7_platform/hw/artyz7_wrapper.xsa", mem_ranges=[0x40000000, 0xbfffffff])

session.configparams("force-mem-accesses", 1)
session.targets("-sn", filter=f"(name =~ *A9*#0) and {CABLE} and {PS_DAP}")
session.rst(type="processor")
session.dow(f"{SW}/artyz7_platform/export/artyz7_platform/sw/boot/fsbl.elf")
bp = session.bpadd(addr="&FsblHandoffJtagExit")
session.con("-b", timeout=60)
bp.remove()

session.targets("-sn", filter=f"(name =~ *A9*#0) and {CABLE} and {PS_DAP}")
session.rst(type="processor")
session.dow(f"{SW}/hello_world/build/hello_world.elf")

session.targets("-sn", filter=f"(name =~ *A9*#1) and {CABLE} and {PS_DAP}")
session.rst(type="processor")
session.dow(f"{SW}/hello_world_1/build/hello_world_1.elf")

# DDR keeps its contents across resets, so clear the handshake words a previous run
# left in the shared section (ready, ack, msg_head, msg_tail in shared_mem/shared_mem.h).
# In a QSPI boot A9#1 waits in the BootROM until hello_world releases it, after clearing
# them; here xsdb starts A9#1 directly, and it would act on a stale ready flag (and
# stale mailbox counters) before A9#0 gets to clear them.
session.mwr(address=0x0FF00004, words=[0, 0, 0, 0])

# Load both cores before running either: hello_world releases A9#1 to 0x10000000
# (0xFFFFFFF0 + sev) right away, which must already hold hello_world_1.
session.targets("-sn", filter=f"(name =~ *A9*#0) and {CABLE} and {PS_DAP}")
session.con()
session.targets("-sn", filter=f"(name =~ *A9*#1) and {CABLE} and {PS_DAP}")
session.con()
session.configparams("force-mem-accesses", 0)

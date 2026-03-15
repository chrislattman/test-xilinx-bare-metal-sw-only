import time
import xsdb

session = xsdb.start_debug_session()
# session.connect(host="173.673.3.2", port=3121) if remote running to hw_server at 173.673.3.2:3121
session.connect()
session.bpremove("--all")
session.targets(id=1)
session.rst()
time.sleep(3)

# To absolutely no one's surprise: AMD put out a half-baked API
# The filter below doesn't work here despite being copied directly from debug.tcl (which works)
session.targets("-s", filter='jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && level==0 && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-23727093-0"')
session.fpga(file="./hello_world/_ide/bitstream/artyz7_wrapper.bit")
session.targets("-sn", filter='name =~"APU*" && jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-4ba00477-0"')
session.loadhw(hw="./artyz7_platform/export/artyz7_platform/hw/artyz7_wrapper.xsa", mem_ranges=[0x40000000, 0xbfffffff])

session.configparams("force-mem-access", 1)
session.targets("-sn", filter='name =~ "*A9*#0" && jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-4ba00477-0"')
session.rst(type="processor")
session.dow("./artyz7_platform/export/artyz7_platform/sw/boot/fsbl.elf")
bp = session.bpadd(addr="FsblHandoffJtagExit")
session.con("-b", timeout=60)
session.bpremove(bp_ids=[bp])

session.targets("-sn", filter='name =~ "*A9*#0" && jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-4ba00477-0"')
session.rst(type="processor")
session.dow("./hello_world/build/hello_world.elf")
session.con()
session.configparams("force-mem-access", 0)

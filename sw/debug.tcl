connect
bpremove -all
targets 1
rst
after 3000

targets -set -filter {jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && level==0 && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-23727093-0"}
fpga ./hello_world/_ide/bitstream/artyz7_wrapper.bit
targets -set -nocase -filter {name =~"APU*" && jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-4ba00477-0"}
loadhw -hw /home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/export/artyz7_platform/hw/artyz7_wrapper.xsa -mem-ranges [list {0x40000000 0xbfffffff}]

configparams force-mem-access 1
targets -set -nocase -filter {name =~ "*A9*#0" && jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-4ba00477-0"}
rst -processor
dow /home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/export/artyz7_platform/sw/boot/fsbl.elf
set bp_1_46_fsbl_bp [bpadd -addr &FsblHandoffJtagExit]
con -block -timeout 60
bpremove $bp_1_46_fsbl_bp

targets -set -nocase -filter {name =~ "*A9*#0" && jtag_cable_name =~ "Digilent Arty Z7 003017BB13CFA" && jtag_device_ctx=="jsn-Arty Z7-003017BB13CFA-4ba00477-0"}
rst -processor
dow /home/chris/test-xilinx-bare-metal-sw-only/sw/hello_world/build/hello_world.elf
bpadd main
con
configparams force-mem-access 0

import argparse
import vitis
from vitis.component import HostComponent
from vitis.platform_component import Platform

release_mode = False
parser = argparse.ArgumentParser()
parser.add_argument("--release", action="store_true")
parser.add_argument("--no-apps", action="store_true", help="only build the platform (BSPs and FSBL)")
args = parser.parse_args()
if args.release:
    release_mode = True

client = vitis.create_client()
client.update_workspace(".")

platform = client.get_component("artyz7_platform")
assert isinstance(platform, Platform)
# platform.update_hw(hw_design="../fpga/artyz7_wrapper.xsa")  # re-read the XSA if desired
if release_mode:
    bsp_domain_name_0 = platform.list_domains()[1]["domain_name"]
    domain_object_0 = platform.get_domain(bsp_domain_name_0)
    domain_object_0.set_config(option="proc", param="proc_extra_compiler_flags", value="-O2 -Wall -Wextra -fno-tree-loop-distribute-patterns")
    bsp_domain_name_1 = platform.list_domains()[2]["domain_name"]
    domain_object_1 = platform.get_domain(bsp_domain_name_1)
    domain_object_1.set_config(option="proc", param="proc_extra_compiler_flags", value="-O2 -Wall -Wextra -fno-tree-loop-distribute-patterns -DUSE_AMP=1")
# Cores don't share peripherals: A9#0 keeps the global timer (shared by both cores, and
# reset by xiltimer's sleep() init), so A9#1's sleep() uses its own SCU private timer
platform.get_domain("standalone_ps7_cortexa9_1").set_config(
    option="lib", param="XILTIMER_sleep_timer", value="ps7_scutimer_0", lib_name="xiltimer")
platform.build()

if not args.no_apps:
    for app_name in ["hello_world", "hello_world_1"]:
        app = client.get_component(app_name)
        assert isinstance(app, HostComponent)
        if release_mode:
            app.set_app_config(key="USER_COMPILE_DEBUG_LEVEL", values="")
            app.set_app_config(key="USER_COMPILE_OPTIMIZATION_LEVEL", values="-O2")
        app.build()

vitis.dispose()

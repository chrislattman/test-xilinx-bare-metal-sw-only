import argparse
import vitis

release_mode = False
parser = argparse.ArgumentParser()
parser.add_argument("--release", action="store_true")
args = parser.parse_args()
if args.release:
    release_mode = True

client = vitis.create_client()
client.update_workspace(".")

platform = client.get_component("artyz7_platform")
if release_mode:
    bsp_domain_name = platform.list_domains()[1]["domain_name"]
    domain_object = platform.get_domain(bsp_domain_name)
    domain_object.set_config(option="proc", param="proc_extra_compiler_flags", value="-O2 -Wall -Wextra -fno-tree-loop-distribute-patterns")
platform.build()

app = client.get_component("hello_world")
if release_mode:
    app.set_app_config(key="USER_COMPILE_DEBUG_LEVEL", values="")
    app.set_app_config(key="USER_COMPILE_OPTIMIZATION_LEVEL", values="-O2")
app.build()

vitis.dispose()

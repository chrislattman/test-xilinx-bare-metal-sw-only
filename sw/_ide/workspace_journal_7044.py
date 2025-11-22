# 2025-11-21T17:10:12.993740700
import vitis

client = vitis.create_client()
client.set_workspace(path="sw")

platform = client.get_component(name="artyz7_platform")
status = platform.build()

comp = client.get_component(name="hello_world")
comp.build()

status = platform.build()

comp.build()

status = platform.build()

comp.build()

comp.build()

comp.build()

comp.build()

vitis.dispose()


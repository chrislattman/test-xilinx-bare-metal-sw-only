# 2025-11-22T17:12:49.715845
import vitis

client = vitis.create_client()
client.set_workspace(path="sw")

platform = client.get_component(name="artyz7_platform")
status = platform.build()

comp = client.get_component(name="hello_world")
comp.build()


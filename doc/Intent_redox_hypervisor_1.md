I want to implement a simple hypervisor (type 1),  microkernel os in rust. We will use the redox os as our primary resource and architecture.

The primary repository for redox is https://github.com/orgs/redox-os

The primrary documentation for redox is https://doc.redox-os.org/book - please read all of this to understand the architecture and the build process.

Please research how to implement a type 1 hypervisor x86, aarch64 and riscv-64 (and provide qemu support), ([xvisor](https://github.com/xvisor/xvisor) is a good reference)

There is currently no hypervisor/virtualisation in redox - we need to implement the hypervisor and then provide a minimum build config which demonstrates this capability. Let's use virtio as our initial architecture for the hypervisor.

>>>>>>>>>>> set this up in https://github.com/ockham-os/ockham
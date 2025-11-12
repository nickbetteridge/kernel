I want to add a type 1 hypervisor to the redox os project.

The primary repository for redox is https://github.com/orgs/redox-os

The primrary documentation for redox is https://doc.redox-os.org/book - please read all of this to understand the architecture and the build process.

Please research how to implement a type 1 hypervisor x86, aarch64 and riscv-64 (and provide qemu support) -[xvisor](https://github.com/xvisor/xvisor) is an excellent reference - we could use the approach contained in the xvisor/arch directory for initialising x86,aarch64 and riscv-64, and get suggestions from the xvisor/core directory for running the scheduler and managing the other kernel components for running the virtualised processes. Please read https://deepwiki.com/xvisor/xvisor/1-overview - this gives a very good overview of xvisor.

I have cloned the following redox libraries - if you need to modify more libraries then just ask me to clone them

https://github.com/nickbetteridge/redox
https://github.com/nickbetteridge/bootloader
https://github.com/nickbetteridge/kernel
https://github.com/nickbetteridge/drivers
https://github.com/nickbetteridge/relibc
https://github.com/nickbetteridge/redoxfs
https://github.com/nickbetteridge/installer

Amongst other things, we will need to initialise/modify the interrupts, enhance the page tables, enhance the cpu spinners, ensure that that there is a fine level of control over the cores and threads and provide a user tender for starting, stopping, restarting, destroying virtual os's and unikernels.

Implement a plan for us to add a type 1 hypervisor to redox os.
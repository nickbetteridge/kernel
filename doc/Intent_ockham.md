I want to create a bare metal, ocaml microkernel. The architectures that will be supported are intel/amd64, aarch64 and riscv64. We should also be able to use qemu for development. 'limine' will be used as the bootloader and will boot into an ocaml runtime. The ocaml runtime will boot into the main scheduler controlling all of the io/driver tasks - we will use picos (github.com/ocaml-multicore/picos) for scheduling. 

https://www.redox-os.org/
https://www.redox-os.org/docs/
https://doc.redox-os.org/book/

rust -> ocaml?

In order to build the ocaml kernel runtime, we need to create a runtime for the bare-metal

The ocaml source (https://github.com/ocaml/ocaml), which when built, produces a runtime for various backends. In our case we will be building a bare metal backend, so the best strategy would be to take the ocaml/runtime source and refactor it for our drivers - are the drivers posix, virtualised or hard-wired?

https://github.com/Solo5/solo5
https://github.com/mirage/ocaml-solo5

Need
  1. micro kernel
  2. acpi/devicetree/dtb/fdt - passed into ocaml kernel
  2. strategy for launching (local) ocaml processes - each with a runtime - virtualised? posix interface?
  1. 
  1. virtualisation
  1. pcie, 

Based on redox

Use :
  1. bootloader

Create :
  1. redox fs
  1. image builder

Useful additions:
  0. port ocaml
  1. port opam
  2. port dune


Make all processes virtual and use one scheduler for the important ones - ie drivers, which put in-coming onto a kernel data stack and outgoing on another kernel data stack - another another scheduler looking after virtual user processes and possibly a third looking after local processes

We might get better parallel domain and thread control using oxcaml for the schedulers

Create two types of unikernels - a standard one for operations such as a server, a user one for interacting with the kernel schemes for local commands and scripts

Do two kernel backendss - one ocaml, one oxcaml, will need to do a modified picos too - picox?




  READ THE BOOK : https://doc.redox-os.org/book



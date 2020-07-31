# RustOS

This OS started as a class project for [CS 3210](https://tc.gts3.org/cs3210/2020/spring/info.html) at Georgia Tech, but I have continued developing new features.

Class Pieces:
* Bootloader
* GPIO
* UART Shell 
* FAT32
* Exception Handlers
* Preemptive Scheduling
* Virtual Memory Management
* Loading user-space processes

Pieces I developed:
* Multi-core support (boot the other 3 cores on the RPi 3)*
* Suspend/Resume, process affinity, process scheduling statistics
* Ethernet (using [USPi](https://github.com/rsta2/uspi))*
* Custom network stack including ARP, IPv4, TCP and ICMP echo
* Telnet Shell
* Interface to the RPi DMA devices
* Initializing the frame buffer and mirroring shell to screen
* Hypervisor using the ARMv8 virtualization exception level
* Virtualized interrupt controllers timers, and UART for the hypervisor guests
* Virtualized NIC and [fork of USPi](https://github.com/Pear0/uspi) that enables pass-through of MAC addresses so that hypervisor guests can act as different network devices 
* Lock registry to enable runtime inspection of locks and statistics tracking
* Timer-based profiling of kernel/hypervisor and guest
* Symbolification of kernel/hypervisor when viewing profiling results by processing DWARF symbols using [gimli](https://github.com/gimli-rs/gimli)
* (very early stages) Support for [Khadas VIM3](https://www.khadas.com/vim3), a Pi-like board but much more powerful 

\* Items with an asterisk were added to the CS 3210 course curriculum after I had added the features

ROOT := $(shell git rev-parse --show-toplevel)

KERN := kernel
TARGET := target/aarch64-unknown-none/release/${KERN}
SDCARD ?= $(ROOT)/ext/fat32-imgs/mock1.fat32.img
# SDCARD ?= $(ROOT)/user/fs.img
OBJCPY := $(HOME)/.cargo/bin/rust-objcopy --strip-all -O binary
TTY_PATH ?= $(shell find /dev -name 'ttyUSB*' -or -name 'cu.usbserial-14*' -maxdepth 1 2>/dev/null | head -n 1)
QEMU_ARGS ?= -smp 4
QEMU_GUI_ARGS ?= -display cocoa
# -net nic,model=virtio

.PHONY: all build qemu transmit objdump nm check clean install test

all: build

build:
	@echo "+ Building build/$(KERN).elf [xbuild/$@]"
	@cargo xbuild --release
	@mkdir -p build
	@cp -f $(TARGET) build/$(KERN).elf

	@echo "+ Building build/$(KERN).bin [objcopy]"
	@$(OBJCPY) $(TARGET) build/$(KERN).bin

check:
	@cargo xcheck

qemu: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS)

qemu-gui: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS) $(QEMU_GUI_ARGS)

qemu-gdb: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS) -s -S

qemu-asm: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS) -d in_asm

transmit: build
	@echo "+ Transmitting build/$(KERN).bin to $(TTY_PATH)"
	ttywrite -i build/$(KERN).bin $(TTY_PATH)
	# screen $(TTY_PATH) 115200
	(stty -f $(TTY_PATH) 115200 >/dev/null | cat $(TTY_PATH) ) 2>/dev/null

transmit_fast: build
	@echo "+ Transmitting build/$(KERN).bin to $(TTY_PATH) @ 921600 baud"
	ttywrite --baud 921600 -i build/$(KERN).bin $(TTY_PATH)
	# screen $(TTY_PATH) 115200
	(stty -f $(TTY_PATH) 115200 >/dev/null | cat $(TTY_PATH) ) 2>/dev/null

objdump: build
	cargo objdump -- -disassemble -no-show-raw-insn -print-imm-hex build/$(KERN).elf

nm: build
	cargo nm build/$(KERN).elf

clean:
	cargo clean
	rm -rf build

install: build
	@echo "+ Installing build/$(KERN).elf [install-kernel.py]"
	@$(ROOT)/bin/install-kernel.py build/$(KERN).elf

test:
	cargo test --target=$(shell $(ROOT)/bin/get-host-target.sh)

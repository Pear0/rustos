ROOT := $(shell git rev-parse --show-toplevel)

KERN := kernel
TARGET := $(ROOT)/target/aarch64-unknown-none/release/${KERN}
TAR := aarch64-unknown-none
# SDCARD ?= $(ROOT)/ext/fat32-imgs/hyper.img
SDCARD ?= kern-image/kern.img
OBJCPY := $(HOME)/.cargo/bin/rust-objcopy --strip-all -O binary
TTY_PATH ?= $(shell find /dev -name 'ttyUSB*' -or -name 'cu.usbserial-14*' -maxdepth 1 2>/dev/null | head -n 1)
QEMU_ARGS ?=
QEMU_GUI_ARGS ?= -display cocoa
TRACE_FILE ?= kernel_trace.log
# -net nic,model=virtio -smp 4

.PHONY: all build build_uimage qemu transmit objdump nm check clean install test img_build

all: build

build:
	@echo "+ Building build/$(KERN).elf [xbuild/$@]"
	@cargo build --target $(TAR) --release
	@mkdir -p build
	@echo "+ Post processing build/$(KERN).elf [post_process_elf.py]"
	@python3 $(ROOT)/bin/post_process_elf.py $(TARGET)

	# force an atomic rename to build/$(KERN).elf for anything
	# watching the file.
	@cp -f $(TARGET) build/$(KERN).elf.new
	@mv -f build/$(KERN).elf.new build/$(KERN).elf

	@echo "+ Building build/$(KERN).bin [objcopy]"
	@$(OBJCPY) build/$(KERN).elf build/$(KERN).bin

	# mkimage -A arm64 -C none -f auto -a 0x80000 -e 0x80000 -n 'kernel' -d build/$(KERN).bin:build/$(KERN).elf build/uImage
	gzip -fk build/$(KERN).elf

build_uimage: build
	mkimage -f uboot_img.its build/uImage

check:
	@cargo check

img_build: build
	cp build/$(KERN).elf kern-image/root/
	cp build/$(KERN).bin kern-image/root/
	kern-image/build-image.sh

qemu: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS)

qemu_no_build:
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS)

clean_qemu:


qemu-trace: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd -trace enable=guest_mem_before_exec,file=$(TRACE_FILE) $(QEMU_ARGS)

qemu-gui: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS) $(QEMU_GUI_ARGS)

qemu-gdb: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS) -s -S

qemu-asm: build
	./qemu.sh build/$(KERN).bin -drive file=$(SDCARD),format=raw,if=sd $(QEMU_ARGS) -d in_asm

transmit2: build
	@echo "+ Transmitting build/$(KERN).bin to $(TTY_PATH)"
	ttywrite -i build/$(KERN).bin $(TTY_PATH)
	screen -A $(TTY_PATH) 115200

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

screen:
	screen -a -A $(TTY_PATH) 115200

objdump:
	$(HOME)/.cargo/bin/rust-objdump --disassemble --no-show-raw-insn --print-imm-hex build/$(KERN).elf

nm: build
	cargo nm build/$(KERN).elf

clean:
	cargo clean
	rm -rf build

install: build_uimage
	@echo "+ Installing build/$(KERN).elf [install-kernel.py]"
	@$(ROOT)/bin/install-kernel.py build/$(KERN).elf - build/uImage ./uboot.ini build/$(KERN).elf.gz build/$(KERN).elf

minstall: build_uimage
	@udiskie-mount -a
	@echo "+ Installing build/$(KERN).elf [install-kernel.py]"
	@$(ROOT)/bin/install-kernel.py build/$(KERN).elf - build/uImage ./uboot.ini build/$(KERN).elf.gz build/$(KERN).elf

test:
	cargo test --target=$(shell $(ROOT)/bin/get-host-target.sh)

doc:
	cargo doc

doc_open:
	cargo doc --open

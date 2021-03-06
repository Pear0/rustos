#!/bin/sh

TOP=$(git rev-parse --show-toplevel)
# export PATH="$TOP/bin:$PATH"

# Delete any old qemu trace files
find . -name 'trace-[0-9]*' -maxdepth 1 -mtime +1 -delete

qemu_bin="qemu-system-aarch64"


qemu_bin="$HOME/Work/qemu/qemu/build/aarch64-softmmu/qemu-system-aarch64"

"$qemu_bin" \
    -nographic \
    -M raspi3 \
    -semihosting \
    -netdev user,id=u1 -net nic,netdev=u1,model=xgmac -object filter-dump,id=f1,netdev=u1,file=/tmp/qemu-nic.pcap \
    -serial null -serial mon:stdio \
    -kernel \
    "$@"

#  --trace events=./qemu/trace-events.txt \


#tap,ifname=tap7,script=./qemu/net_up.sh,downscript=./qemu/net_down.sh,id=u1
# -netdev user,id=u1 -net nic,netdev=u1,model=xgmac -object filter-dump,id=f1,netdev=u1,file=/tmp/qemu-nic.pcap \

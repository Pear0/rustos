#!/bin/sh

TOP=$(git rev-parse --show-toplevel)

# export PATH="$TOP/bin:$PATH"

qemu_bin="qemu-system-aarch64"


qemu_bin="$HOME/Work/qemu/qemu/build/aarch64-softmmu/qemu-system-aarch64"

"$qemu_bin" \
    -nographic \
    -M raspi3 \
    --trace events=./qemu/trace-events.txt \
    -netdev user,id=u1 -net nic,netdev=u1,model=xgmac -object filter-dump,id=f1,netdev=u1,file=/tmp/qemu-nic.pcap \
    -serial null -serial mon:stdio \
    -kernel \
    "$@"

#tap,ifname=tap7,script=./qemu/net_up.sh,downscript=./qemu/net_down.sh,id=u1
# -netdev user,id=u1 -net nic,netdev=u1,model=xgmac -object filter-dump,id=f1,netdev=u1,file=/tmp/qemu-nic.pcap \

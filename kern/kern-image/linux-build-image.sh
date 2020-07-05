#!/bin/bash
set -e

img_file=kern.img
# 512 MB
img_size=536870912
#
#if [ ! -f "$img_file" ]; then
#  echo 'Allocating file...'
#  fallocate -v -x -l "$img_size" "$img_file"
#fi

sfdisk -d "hyper.img"

#echo 'type=b, bootable' | sfdisk "$img_file"
#
#file "$img_file"
#
#df -h
#
#echo 'Mounting loop...'
#losetup -P -f --show "$img_file"
#
#
#ls /dev

#echo 'Building filesystem...'
#mkfs.fat -n 'KERN' -F 32 /dev/loop5p1
#file "$img_file"

echo 'Copying...'

cp hyper.img "$img_file"

echo 'Mounting...'

mkdir /img_mount
mount -o loop,offset=$((1 * 512)) "$img_file" /img_mount

cp -r root/* /img_mount || echo 'no files to copy!'

echo 'File tree'
tree /img_mount

echo 'Unmounting...'
umount /img_mount

chmod 666 "$img_file"


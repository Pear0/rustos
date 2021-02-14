#!/usr/bin/env python3
from __future__ import print_function
import sys

from elftools.elf.elffile import ELFFile
from elftools.elf.sections import NoteSection

build_id_placeholder = b'\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\x0c\r\x0e\x0f\x10'


def process_file(filename):
    print('[post] Post Processing file:', filename)
    with open(filename, 'rb') as f:
        elf_file = ELFFile(f)
        build_id_hex = None
        for sect in elf_file.iter_sections():
            if not isinstance(sect, NoteSection):
                continue
            for note in sect.iter_notes():
                if note['n_type'] == 'NT_GNU_BUILD_ID':
                    build_id_hex = note['n_desc']

        print('[post] found build id:', build_id_hex)

        build_id = bytes.fromhex(build_id_hex)

        if len(build_id) > len(build_id_placeholder):
            build_id = build_id[:len(build_id_placeholder)]

        symbol_table = elf_file.get_section_by_name('.symtab')

        symbols = symbol_table.get_symbol_by_name('BUILD_ID')
        if not symbols:
            print('[post] cannot find BUILD_ID symbol')
            return
        assert len(symbols) == 1
        symbol = symbols[0]

        build_id_addr, build_id_size = symbol['st_value'], symbol['st_size']
        print('[post] BUILD_ID has address {}, size {}'.format(build_id_addr, build_id_size))

        file_offsets = list(elf_file.address_offsets(build_id_addr, build_id_size))
        assert len(file_offsets) == 1
        file_offset = file_offsets[0]

        print('[post] BUILD_ID has file offset {}'.format(file_offset))

    with open(filename, 'r+b') as f:
        f.seek(file_offset)
        existing_build_id = f.read(len(build_id))
        if existing_build_id == build_id:
            print('[post] BUILD_ID = {}, already inserted'.format(build_id.hex()))
            return
        if existing_build_id != build_id_placeholder:
            print('[post] BUILD_ID has unknown value {}, refusing to overwrite'.format(existing_build_id.hex()))
            return

        print('[post] BUILD_ID {} -> {}, replaced'.format(existing_build_id.hex(), build_id.hex()))
        f.seek(file_offset)
        f.write(build_id)


if __name__ == '__main__':
    process_file(sys.argv[1])

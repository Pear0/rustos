
use dtb;
use hex::ToHex;
use pretty_hex::PrettyHex;

#[repr(C, align(4))]
struct MyAlignedBuffer([u8; 4 * 16]);

impl MyAlignedBuffer {
    pub fn new() -> Self {
        MyAlignedBuffer([0; 64])
    }
}

pub unsafe fn dump_dtb(addr: u64) {

    let reader = match dtb::Reader::read_from_address(addr as usize) {
        Ok(reader) => reader,
        Err(e) => {
            kprintln!("Error reading: {:?}", e);
            return;
        }
    };

    kprintln!("Reserved Mem:");
    for entry in reader.reserved_mem_entries() {
        kprintln!("  {:#x} - {:#x}", entry.address, entry.address + entry.size - 1);
    }

    let mut indent = 0;
    let root = reader.struct_items();
    for node in root {
        match &node {
            dtb::StructItem::BeginNode { name, .. } => {
                for _ in 0..indent { kprint!(" "); }
                kprintln!("node: {}", name);
                indent += 2;

            }
            dtb::StructItem::Property { name, .. }  => {
                for _ in 0..indent { kprint!(" "); }
                kprintln!("  property: {}", name);
            }
            dtb::StructItem::EndNode { .. } => {
                indent -= 2;
            },
        }
    }

    let root = reader.struct_items();

    let mut scratch = MyAlignedBuffer::new();

    let (prop, _d) =
        root.path_struct_items("/memory/reg").next().unwrap();
    kprintln!("property: {:?}\n {:?}", prop.name(), prop.value_u32_list(&mut scratch.0));

    let (prop, _) =
        root.path_struct_items("/memory/device_type").next().unwrap();
    kprintln!("property: {:?}\n {:?}", prop.name(), prop.value().unwrap_or(&[]).hex_dump());


}




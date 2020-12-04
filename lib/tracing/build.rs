use capnpc;

fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/tracestreaming.capnp")
        .run().expect("schema compiler command");
}

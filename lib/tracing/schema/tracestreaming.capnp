@0xab604b6765b5f571;

using Cxx = import "/capnp/c++.capnp";
$Cxx.namespace("net_trace");

struct TraceGroup @0x9446c7cee4aba8ae {
    events @0 :List(TraceEvent);
    buildId @1 :UInt64;
}

struct TraceEvent @0xc84aca40859a2728 {
    timestamp @0 :UInt64;
    eventIndex @1 :UInt64;
    frames @2 :List(TraceFrame);
}

struct TraceFrame @0x83b890232f7ef4c1 {
    pc @0 :UInt64;
}

struct SavedTraces @0xd194ad8ff09881cf {
    groups @0 :List(TraceGroup);
}

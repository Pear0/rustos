project(cs3210)
cmake_minimum_required(VERSION 3.15)

#Rust part start
set(RUST_MODULE_NAME f3blink)
set(RUST_MODULE_ROOT ${CMAKE_CURRENT_SOURCE_DIR})
set(RUST_LIB ${RUST_MODULE_ROOT}/build/kernel.elf)
set_source_files_properties(
        ${RUST_LIB}
        PROPERTIES
        EXTERNAL_OBJECT true
        GENERATED true
)

set_property(DIRECTORY APPEND
        PROPERTY ADDITIONAL_MAKE_CLEAN_FILES ${RUST_MODULE_ROOT}/target
        )

add_custom_command(OUTPUT ${RUST_LIB}
        COMMAND cargo xbuild --release
        DEPENDS ${RUST_MODULE_ROOT}/src/*.* ${RUST_MODULE_ROOT}/Cargo.*
        COMMENT "Compiling rust"
        WORKING_DIRECTORY ${RUST_MODULE_ROOT}
        USES_TERMINAL
        )
#Rust part end

add_custom_target(kern ALL
        COMMENT "Compiling client module"
        COMMAND cargo xbuild --release
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

set_target_properties(kern PROPERTIES LINK_DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/.cargo/layout.ld)

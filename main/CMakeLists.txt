idf_component_register()

# Get all of the include directories for all the components we're building, so
# that it's easy to run `bindgen` on them later.
foreach(req ${BUILD_COMPONENTS})
    idf_component_get_property(req_lib "${req}" COMPONENT_LIB)
    get_target_property(target_type ${req_lib} TYPE)
    if (NOT target_type STREQUAL "INTERFACE_LIBRARY")
        get_target_property(target_includes "${req_lib}" INCLUDE_DIRECTORIES)
        foreach(dir ${target_includes})
            string(APPEND all_includes "-I${dir} ")
        endforeach()
    endif ()
endforeach()

# Write the resulting value out to a file for later use. We don't run `bindgen`
# automatically as part of the build.
file(GENERATE OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/main_includes.txt CONTENT ${all_includes})

externalproject_add(esp32_rust_main_build
    PREFIX ${COMPONENT_DIR}/target
    SOURCE_DIR ${COMPONENT_DIR}
    CONFIGURE_COMMAND ""
    BUILD_ALWAYS 1
    BUILD_BYPRODUCTS "${COMPONENT_DIR}/target/xtensa-esp32-none-elf/release/libesp32_hello_rust_main.a"
    BUILD_ALWAYS 1
    BUILD_COMMAND cargo +xtensa xbuild --target xtensa-esp32-none-elf
    COMMAND cargo +xtensa xbuild --target xtensa-esp32-none-elf --release
    INSTALL_COMMAND ""
    LOG_BUILD ON
    )

add_library(rust_main STATIC IMPORTED GLOBAL)
add_dependencies(rust_main esp32_rust_main_build)

set_property(TARGET rust_main APPEND PROPERTY LINK_LIBRARIES idf::freertos)
set_property(TARGET rust_main APPEND PROPERTY INTERFACE_LINK_LIBRARIES idf::freertos)
set_property(TARGET rust_main APPEND PROPERTY LINK_LIBRARIES idf::esp32)
set_property(TARGET rust_main APPEND PROPERTY INTERFACE_LINK_LIBRARIES idf::esp32)

set_target_properties(rust_main PROPERTIES IMPORTED_LOCATION
  ${COMPONENT_DIR}/target/xtensa-esp32-none-elf/release/libesp32_hello_rust_main.a)

set_target_properties(rust_main PROPERTIES INTERFACE_INCLUDE_DIRECTORIES
     ${COMPONENT_DIR})

target_link_libraries(${COMPONENT_LIB} INTERFACE rust_main)

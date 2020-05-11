#!/usr/bin/env bash

set -eu

INCLUDES=`cat ../build/esp-idf/main/main_includes.txt`
COMPS=$IDF_PATH/components
: "${SYSROOT:=$($XTENSA_LLVM_ROOT/bin/clang --print-resource-dir)}"
TARGET=xtensa-esp32-none-elf
: "${BINDGEN:=bindgen}"
: "${LIBCLANG_PATH:=$XTENSA_LLVM_ROOT/lib/}"
CLANG_FLAGS="\
    --sysroot=$SYSROOT \
    $INCLUDES \
    -I"$COMPS/newlib/include" \
    -I"$(pwd)" \
    -D__bindgen \
    --target=$TARGET \
    -x c"


generate_bindings()
{
    # --no-rustfmt-bindings because we run rustfmt separately with regular rust
    LIBCLANG_PATH="$LIBCLANG_PATH" \
    "$BINDGEN" \
        --use-core \
        --ctypes-prefix crate::types \
        --no-layout-tests \
        --no-rustfmt-bindings \
        --output esp-idf-sys/src/bindings.rs \
        esp-idf-sys/src/bindings.h \
        -- $CLANG_FLAGS

    rustup run stable rustfmt esp-idf-sys/src/bindings.rs
}

generate_bindings "$@"


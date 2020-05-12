#!/usr/bin/env bash

set -eu

INCLUDES=$(cat ../build/esp-idf/main/main_includes.txt | python -c "import os.path, sys; print(' '.join(['-I' + os.path.relpath(s.strip()[1:-1], '..') for s in sys.stdin.readlines()]))")

: "${SYSROOT:=$(docker run --rm rbtying/esp-crossbuild-env /opt/llvm-xtensa/bin/clang --print-resource-dir)}"
TARGET=xtensa-esp32-none-elf
CLANG_FLAGS="\
    --sysroot=$SYSROOT \
    $INCLUDES \
    -I"/esp-idf/components/newlib/include" \
    -I"$(pwd)" \
    -D__bindgen \
    --target=$TARGET \
    -x c"


generate_bindings()
{
    # --no-rustfmt-bindings because we run rustfmt separately with regular rust
    docker run --rm --mount type=bind,source=$IDF_PATH,target=/esp-idf --mount type=bind,source=$(pwd)/..,target=/project rbtying/esp-crossbuild-env bindgen \
        --use-core \
        --ctypes-prefix crate::types \
        --no-layout-tests \
        --no-rustfmt-bindings \
        --output main/esp-idf-sys/src/bindings.rs \
        main/esp-idf-sys/src/bindings.h \
        -- $CLANG_FLAGS

    docker run --rm --mount type=bind,source=$(pwd),target=/project rbtying/esp-crossbuild-env rustup run stable rustfmt esp-idf-sys/src/bindings.rs
}

generate_bindings "$@"


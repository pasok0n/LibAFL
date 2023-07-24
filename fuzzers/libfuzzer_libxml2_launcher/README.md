# Libfuzzer for libxml2-2.9.14, with launcher

This folder contains an example fuzzer for libxml2, using LLMP for fast multi-process fuzzing and crash detection.
It has been tested on Linux.

This uses the `launcher` feature, that automatically spawns `n` child processes, and binds them to a free core.

## Build

To build this example, run:

```bash
cargo build --release
```

This will build the library with the fuzzer (src/lib.rs) with the libfuzzer compatibility layer and the SanitizerCoverage runtime functions for coverage feedback.
In addition, it will also build two C and C++ compiler wrappers (bin/libafl_c(libafl_c/xx).rs) that you must use to compile the target.

Then download libxml2, and unpack the archive:
```bash
wget https://download.gnome.org/sources/libxml2/2.9/libxml2-2.9.14.tar.xz
tar -xvf libxml2-2.9.14.tar.xz
```

Now compile libxml2, using the libafl_cc compiler wrapper:

```bash
cd libxml2-2.9.14
./configure --disable-shared --without-debug --without-ftp --without-http --without-legacy --without-python LIBS='-ldl'
make -C libxml2-2.9.14 CC=../target/release/libafl_cc CXX=../target/release/libafl_cxx -j `nproc`
```

You can find the static lib at `libxml2-2.9.14/.libs/libxml2.a`.

Now, we have to build the libfuzzer harness and link all together to create our fuzzer binary.

```
cd ..
./target/release/libafl_cxx ./harness.cc libxml2-2.9.14/.libs/libxml2.a -I libxml2-2.9.14/include/ -I libxml2-2.9.14/ -o fuzzer_libxml2 -lm -lz -llzma
```

Afterwards, the fuzzer will be ready to run.

Alternatively you can run `cargo make run` and this command will automatically build and run the fuzzer

## Run

Just run once, the launcher feature should do the rest.
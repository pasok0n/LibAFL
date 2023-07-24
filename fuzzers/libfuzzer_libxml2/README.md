# Libfuzzer for libxml2-2.9.14

This folder contains an example fuzzer for libxml2, using LLMP for fast multi-process fuzzing and crash detection.

In contrast to other fuzzer examples, this setup uses `fuzz_loop_for`, to occasionally respawn the fuzzer executor.
While this costs performance, it can be useful for targets with memory leaks or other instabilities.
If your target is really instable, however, consider exchanging the `InProcessExecutor` for a `ForkserverExecutor` instead.

It also uses the `introspection` feature, printing fuzzer stats during execution.

To show off crash detection, we added a `ud2` instruction to the harness, edit harness.cc if you want a non-crashing example.
It has been tested on Linux.

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

The first time you run the binary, the broker will open a tcp port (currently on port `1337`), waiting for fuzzer clients to connect. This port is local and only used for the initial handshake. All further communication happens via shared map, to be independent of the kernel. Currently, you must run the clients from the libfuzzer_libxml2 directory for them to be able to access the corpus.

```
./fuzzer_libxml2

[libafl/src/bolts/llmp.rs:407] "We're the broker" = "We\'re the broker"
Doing broker things. Run this tool again to start fuzzing in a client.
```

And after running the above again in a separate terminal:

```
[libafl/src/bolts/llmp.rs:1464] "New connection" = "New connection"
[libafl/src/bolts/llmp.rs:1464] addr = 127.0.0.1:33500
[libafl/src/bolts/llmp.rs:1464] stream.peer_addr().unwrap() = 127.0.0.1:33500
[LOG Debug]: Loaded 4 initial testcases.
[New Testcase #2] clients: 3, corpus: 6, objectives: 0, executions: 5, exec/sec: 0
< fuzzing stats >
```

As this example uses in-process fuzzing, we added a Restarting Event Manager (`setup_restarting_mgr`).
This means each client will start itself again to listen for crashes and timeouts.
By restarting the actual fuzzer, it can recover from these exit conditions.

In any real-world scenario, you should use `taskset` to pin each client to an empty CPU core, the lib does not pick an empty core automatically (yet).
#[cfg(windows)]
use std::ptr::write_volatile;
use core::time::Duration;
use std::{env, net::SocketAddr, path::PathBuf, ptr::write};
use clap::{self, Parser};
#[cfg(feature = "tui")]
use libafl::monitors::tui::{ui::TuiUI, TuiMonitor};
#[cfg(not(feature = "tui"))]
use libafl::monitors::{MultiMonitor, OnDiskTOMLMonitor};
use libafl::{
    bolts::{core_affinity::Cores, current_nanos, launcher::Launcher, rands::StdRand, tuples::tuple_list, AsSlice, shmem::{ShMemProvider, StdShMemProvider},},
    corpus::{InMemoryCorpus, OnDiskCorpus},
    executors::{inprocess::InProcessExecutor, ExitKind},
    feedbacks::{CrashFeedback, MaxMapFeedback},
    fuzzer::{Fuzzer, StdFuzzer},
    generators::RandPrintablesGenerator,
    inputs::{BytesInput, HasTargetBytes},
    mutators::scheduled::{havoc_mutations, StdScheduledMutator},
    observers::StdMapObserver,
    schedulers::QueueScheduler,
    stages::mutational::StdMutationalStage,
    state::StdState,
    prelude::EventConfig,
    Error,
};

/// Coverage map with explicit assignments due to the lack of instrumentation
static mut SIGNALS: [u8; 16] = [0; 16];
static mut SIGNALS_PTR: *mut u8 = unsafe { SIGNALS.as_mut_ptr() };

/// Assign a signal to the signals map
fn signals_set(idx: usize) {
    unsafe { write(SIGNALS_PTR.add(idx), 1) };
}
/// Parse a millis string to a [`Duration`]. Used for arg parsing.
fn timeout_from_millis_str(time: &str) -> Result<Duration, Error> {
    Ok(Duration::from_millis(time.parse()?))
}

/// The commandline args this fuzzer accepts
#[derive(Debug, Parser)]
#[command(
    name = "libfuzzer_libpng_launcher",
    about = "A libfuzzer-like fuzzer for libpng with llmp-multithreading support and a launcher",
    author = "Andrea Fioraldi <andreafioraldi@gmail.com>, Dominik Maier <domenukk@gmail.com>"
)]
struct Opt {
    #[arg(
        short,
        long,
        value_parser = Cores::from_cmdline,
        help = "Spawn a client in each of the provided cores. Broker runs in the 0th core. 'all' to select all available cores. 'none' to run a client without binding to any core. eg: '1,2-4,6' selects the cores 1,2,3,4,6.",
        name = "CORES"
    )]
    cores: Cores,

    #[arg(
        short = 'p',
        long,
        help = "Choose the broker TCP port, default is 1337",
        name = "PORT",
        default_value = "1337"
    )]
    broker_port: u16,

    #[arg(short = 'a', long, help = "Specify a remote broker", name = "REMOTE")]
    remote_broker_addr: Option<SocketAddr>,

    #[arg(short, long, help = "Set an initial corpus directory", name = "INPUT")]
    input: Vec<PathBuf>,

    #[arg(
        short,
        long,
        help = "Set the output directory, default is ./out",
        name = "OUTPUT",
        default_value = "./out"
    )]
    output: PathBuf,

    #[arg(
        value_parser = timeout_from_millis_str,
        short,
        long,
        help = "Set the exeucution timeout in milliseconds, default is 10000",
        name = "TIMEOUT",
        default_value = "10000"
    )]
    timeout: Duration,
}

#[allow(clippy::similar_names, clippy::manual_assert)]
pub fn main() {
    let opt = Opt::parse();

    let broker_port = opt.broker_port;
    let cores = opt.cores;

    println!(
        "Workdir: {:?}",
        env::current_dir().unwrap().to_string_lossy().to_string()
    );

    let shmem_provider = StdShMemProvider::new().expect("Failed to init shared memory");
    #[cfg(not(feature = "tui"))]
    let monitor = OnDiskTOMLMonitor::new(
        "./fuzzer_stats.toml",
        MultiMonitor::new(|s| println!("{s}")),
    );

    let mut run_client = |_state: Option<_>, mut restarting_mgr, _core_id| {
        // The closure that we want to fuzz
        let mut harness = |input: &BytesInput| {
            let target = input.target_bytes();
            let buf = target.as_slice();
            signals_set(0);
            if !buf.is_empty() && buf[0] == b'a' {
                signals_set(1);
                if buf.len() > 20 && buf[20] == b'b' {
                    signals_set(2);
                    if buf.len() > 112 && buf[112] == b'4' {
                        signals_set(3);
                        if buf.len() > 120 && buf[120] == b'4' {
                            signals_set(4);
                            if buf.len() > 121 && buf[121] == b'q' {
                                signals_set(5);
                                if buf.len() > 503 && buf[503] == b'9' {
                                    signals_set(6);
                                    if buf.len() > 1059 && buf[1059] == b'z' {
                                        signals_set(7);
                                        if buf.len() > 6433 && buf[6433] == b'o' {
                                            signals_set(8);
                                            if buf.len() > 10059 && buf[10059] == b'#' {
                                                signals_set(9);
                                                if buf.len() > 10069 && buf[10069] == b'_' {
                                                    #[cfg(unix)]
                                                    panic!("Artificial bug triggered =)");

                                                    #[cfg(windows)]
                                                    unsafe {
                                                        write_volatile(0 as *mut u32, 0);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ExitKind::Ok
        };

        // Create an observation channel using the signals map
        let observer = unsafe { StdMapObserver::from_mut_ptr("signals", SIGNALS_PTR, SIGNALS.len()) };

        // Feedback to rate the interestingness of an input
        let mut feedback = MaxMapFeedback::new(&observer);

        // A feedback to choose if an input is a solution or not
        let mut objective = CrashFeedback::new();

        // create a State from scratch
        let mut state = StdState::new(
            // RNG
            StdRand::with_seed(current_nanos()),
            // Corpus that will be evolved, we keep it in memory for performance
            InMemoryCorpus::new(),
            // Corpus in which we store solutions (crashes in this example),
            // on disk so the user can get them after stopping the fuzzer
            OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(),
            // States of the feedbacks.
            // The feedbacks can report the data that should persist in the State.
            &mut feedback,
            // Same for objective feedbacks
            &mut objective,
        )
        .unwrap();

        // The Monitor trait define how the fuzzer stats are displayed to the user

        #[cfg(feature = "tui")]
        let ui = TuiUI::with_version(String::from("Baby Fuzzer"), String::from("0.0.1"), false);

        // A queue policy to get testcasess from the corpus
        let scheduler = QueueScheduler::new();

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        // Create the executor for an in-process function with just one observer
        let mut executor = InProcessExecutor::new(
            &mut harness,
            tuple_list!(observer),
            &mut fuzzer,
            &mut state,
            &mut restarting_mgr,
        )
        .expect("Failed to create the Executor");

        // Generator of printable bytearrays of max size 32
        let mut generator = RandPrintablesGenerator::new(10240);

        // Generate 8 initial inputs
        state
            .generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut restarting_mgr, 8)
            .expect("Failed to generate the initial corpus");

        // Setup a mutational stage with a basic bytes mutator
        let mutator = StdScheduledMutator::new(havoc_mutations());
        let mut stages = tuple_list!(StdMutationalStage::new(mutator));

        fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut restarting_mgr)?;
        Ok(())
    };

    match Launcher::builder()
        .shmem_provider(shmem_provider)
        .configuration(EventConfig::from_name("default"))
        .monitor(monitor)
        .run_client(&mut run_client)
        .cores(&cores)
        .broker_port(broker_port)
        .remote_broker_addr(opt.remote_broker_addr)
        .stdout_file(Some("/dev/null"))
        .build()
        .launch()
    {
        Ok(()) => (),
        Err(Error::ShuttingDown) => println!("Fuzzing stopped by user. Good bye."),
        Err(err) => panic!("Failed to run launcher: {err:?}"),
    }
}

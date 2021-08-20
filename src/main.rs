use ::std::{
    process::{Command, Child},
    sync::mpsc::channel,
    time::Duration,
    ffi::{OsString, OsStr}
};

use ::notify::{
    watcher,
    Watcher,
    RecursiveMode,
    DebouncedEvent
};
use ::structopt::StructOpt;
use ::ignore::gitignore::GitignoreBuilder;

#[derive(StructOpt)]
struct BuildNRun {
    #[structopt(long, help = "Path to watch")]
    watch: Vec<OsString>,
    #[structopt(long, short, help = "No output printed to stdout")]
    quiet: bool,
    #[structopt(long, short, help = "Number of parallel jobs, default to # of CPUs")]
    jobs: Option<i32>,
    #[structopt(long, help = "Build only the specified binary")]
    bin: OsString,
    #[structopt(long, help = "Build artifacts in release mode, with optimizations")]
    release: bool,
    #[structopt(long, help = "Space or comma separated list of features to activate")]
    features: Option<OsString>,
    #[structopt(long, help = "Activate all available features")]
    all_features: bool,
    #[structopt(long, help = "Do not activate the `default` feature")]
    no_default_features: bool,
    #[structopt(long, help = "Directory for all generated artifacts")]
    target_dir: Option<OsString>,
    #[structopt(long, help = "Path to Cargo.toml")]
    manifest_path: Option<OsString>,
    #[structopt(long, help = "Require Cargo.lock and cache are up to date")]
    frozen: bool,
    #[structopt(long, help = "Require Cargo.lock is up to date")]
    locked: bool,
    #[structopt(long, help = "Run without accessing the network")]
    offline: bool,
    run_params: Vec<OsString>
}

fn build(bnr: &BuildNRun) -> bool {
    let mut args = vec![OsStr::new("build"), OsStr::new("--bin"), &bnr.bin];

    if bnr.quiet {
        args.push(OsStr::new("--quiet"));
    }

    let jobs_str;
    if let Some(jobs) = bnr.jobs {
        jobs_str = format!("{}", jobs);
        args.push(OsStr::new("--jobs"));
        args.push(OsStr::new(&jobs_str));
    }

    if bnr.release {
        args.push(OsStr::new("--release"));
    }
    for features in &bnr.features {
        args.push(&features);
    }
    if bnr.all_features {
        args.push(OsStr::new("--all-features"));
    }
    if bnr.no_default_features {
        args.push(OsStr::new("--no-default-features"));
    }
    if let Some(mft) = &bnr.manifest_path {
        args.push(OsStr::new("--manifest-path"));
        args.push(&mft);
    }
    if bnr.frozen {
        args.push(OsStr::new("--frozen"));
    }
    if bnr.locked {
        args.push(OsStr::new("--locked"));
    }
    if bnr.offline {
        args.push(OsStr::new("--offline"));
    }
    Command::new("cargo").args(&args).status().unwrap().success()
}

fn run(bnr: &BuildNRun) -> Option<Child> {
    let mut exe = match &bnr.target_dir {
        Some(td) => td.clone(),
        None => OsString::from("target/")
    };
    if bnr.release {
        exe.push("release");
    } else {
        exe.push("debug");
    }
    exe.push("/");
    exe.push(&bnr.bin);
    Command::new(exe).args(&bnr.run_params).spawn().ok()
}

fn main() {
    let build_n_run = BuildNRun::from_args();

    let (gi, _) = GitignoreBuilder::new(".").build_global();

    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(250)).unwrap();
    if build_n_run.watch.is_empty() {
        watcher.watch(".", RecursiveMode::Recursive).unwrap();
    } else {
        for watch in &build_n_run.watch {
            watcher.watch(watch, RecursiveMode::Recursive).unwrap();
        }
    }

    let mut proc: Option<Child> = None;

    loop {
        if build(&build_n_run) {
            for mut child in proc.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            proc = run(&build_n_run);
        }

        loop {
            match rx.recv().unwrap() {
                DebouncedEvent::Write(f) if !gi.matched(&f, false).is_ignore() => break,
                DebouncedEvent::Rename(_, f) if !gi.matched(&f, false).is_ignore() => break,
                DebouncedEvent::Remove(f) if !gi.matched(&f, false).is_ignore() => break,
                DebouncedEvent::Create(f) if !gi.matched(&f, false).is_ignore() => break,
                DebouncedEvent::Rescan => break,
               _ => continue
            }
        }
        while rx.try_recv().is_ok() {}
    }
}

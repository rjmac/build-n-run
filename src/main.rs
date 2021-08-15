use ::std::{
    process::{Command, Child},
    sync::mpsc::channel,
    time::Duration
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
    #[structopt(long)]
    release: bool,
    run_params: Vec<String>
}

fn build(bnr: &BuildNRun) -> bool {
    let mut args = vec!["build", "--bin", "backend"];
    if bnr.release {
        args.push("--release")
    }
    Command::new("cargo").args(&args).status().unwrap().success()
}

fn run(bnr: &BuildNRun) -> Option<Child> {
    let exe = if bnr.release {
        "target/release/backend"
    } else {
        "target/debug/backend"
    };
    Command::new(exe).args(&bnr.run_params).spawn().ok()
}

fn main() {
    let build_n_run = BuildNRun::from_args();

    let (gi, _) = GitignoreBuilder::new(".").build_global();

    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(250)).unwrap();
    watcher.watch("common", RecursiveMode::Recursive).unwrap();
    watcher.watch("backend", RecursiveMode::Recursive).unwrap();

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

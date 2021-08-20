use ::std::{
    process::{Command, Child},
    sync::mpsc::channel,
    time::Duration,
    env::args_os,
    ffi::OsString
};

use ::clap::{
    crate_name,
    crate_version
};
use ::notify::{
    watcher,
    Watcher,
    RecursiveMode,
    DebouncedEvent
};
use ::ignore::gitignore::GitignoreBuilder;

fn build(build_args: &[OsString]) -> bool {
    Command::new("cargo").arg("build").args(build_args).status().unwrap().success()
}

fn run(build_args: &[OsString], run_args: &[OsString]) -> Option<Child> {
    let exe =
        if build_args.iter().find(|e| *e == "--release").is_some() {
            "target/release/backend"
        } else {
            "target/debug/backend"
        };
    Command::new(exe).args(run_args).spawn().ok()
}

fn main() {
    let args: Vec<_> = args_os().skip(1).collect();
    if args == &["--version"] {
        println!("{} {}", crate_name!(), crate_version!());
        return;
    }
    let (build_args, run_args): (&[_], &[_]) =
        match args.iter().position(|a| a == "--") {
            Some(idx) => {
                let (l, r) = args.split_at(idx);
                (l, &r[1..])
            },
            None => (args.as_slice(), &[])
        };
    let (gi, _) = GitignoreBuilder::new(".").build_global();

    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(250)).unwrap();
    watcher.watch("common", RecursiveMode::Recursive).unwrap();
    watcher.watch("backend", RecursiveMode::Recursive).unwrap();

    let mut proc: Option<Child> = None;

    loop {
        if build(build_args) {
            for mut child in proc.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            proc = run(build_args, run_args);
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

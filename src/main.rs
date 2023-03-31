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
use ::clap::{
    Parser,
    AppSettings
};
use ::ignore::gitignore::GitignoreBuilder;
use ::strum::{IntoStaticStr, EnumString, EnumVariantNames, VariantNames};

#[derive(Clone, Copy, EnumString, EnumVariantNames, IntoStaticStr, Debug)]
#[strum(serialize_all = "lowercase")]
enum Color {
    Auto,
    Always,
    Never
}

#[derive(Parser)]
#[clap(setting(AppSettings::DeriveDisplayOrder), version)]
struct BuildNRun {
    #[clap(long, parse(from_os_str), value_name = "PATH", help = "Path to watch")]
    watch: Vec<OsString>,
    #[clap(long, short, help = "No output printed to stdout")]
    quiet: bool,
    #[clap(long, parse(from_os_str), value_name = "NAME", help = "Build only the specified binary")]
    bin: OsString,
    #[clap(long, short, value_name = "SPEC", help = "Package with the target to run")]
    package: Option<OsString>,
    #[clap(long, short, value_name = "N", help = "Number of parallel jobs, default to # of CPUs")]
    jobs: Option<i32>,
    #[clap(long, help = "Build artifacts in release mode, with optimizations")]
    release: bool,
    #[clap(long, parse(from_os_str), value_name = "PROFILE-NAME", help = "Build artifacts with the specified profile")]
    profile: Option<OsString>,
    #[clap(long, parse(from_os_str), value_name = "FEATURES", help = "Space or comma separated list of features to activate")]
    features: Vec<OsString>,
    #[clap(long, help = "Activate all available features")]
    all_features: bool,
    #[clap(long, help = "Do not activate the `default` feature")]
    no_default_features: bool,
    #[clap(long, parse(from_os_str), value_name = "TRIPLE", help = "Build for the target triple")]
    target: Option<OsString>,
    #[clap(long, parse(from_os_str), value_name = "DIRECTORY", help = "Directory for all generated artifacts")]
    target_dir: Option<OsString>,
    #[clap(long, value_name = "PATH", help = "Path to Cargo.toml")]
    manifest_path: Option<OsString>,
    #[clap(long, parse(from_os_str), value_name = "FMT", help = "Error format")]
    message_format_path: Vec<OsString>,
    #[clap(short, long, parse(from_occurrences), help = "Use verbose output (-vv very verbose/build.rs output)")]
    verbose: u32,
    #[clap(long, possible_values = Color::VARIANTS, value_name = "WHEN", help = "Coloring")]
    color: Option<Color>,
    #[clap(long, help = "Require Cargo.lock and cache are up to date")]
    frozen: bool,
    #[clap(long, help = "Require Cargo.lock is up to date")]
    locked: bool,
    #[clap(long, help = "Run without accessing the network")]
    offline: bool,
    #[clap(parse(from_os_str))]
    args: Vec<OsString>
}

fn build(bnr: &BuildNRun) -> bool {
    let mut args = vec![OsStr::new("build")];

    if bnr.quiet {
        args.push(OsStr::new("--quiet"));
    }

    let mut bin_str = OsStr::new("--bin=").to_owned();
    bin_str.push(&bnr.bin);
    args.push(&bin_str);

    let mut package_str;
    if let Some(package) = &bnr.package {
        package_str = OsStr::new("--package=").to_owned();
        package_str.push(package);
        args.push(&package_str);
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

    let mut profile_str;
    if let Some(profile) = &bnr.profile {
        profile_str = OsStr::new("--profile=").to_owned();
        profile_str.push(profile);
        args.push(&profile_str);
    }

    let mut features_strs = Vec::new();
    for features in &bnr.features {
        let mut features_str = OsStr::new("--features=").to_owned();
        features_str.push(features);
        features_strs.push(features_str);
    }
    for features_str in &features_strs {
        args.push(features_str);
    }

    if bnr.all_features {
        args.push(OsStr::new("--all-features"));
    }

    if bnr.no_default_features {
        args.push(OsStr::new("--no-default-features"));
    }

    let mut target_str;
    if let Some(target) = &bnr.target {
        target_str = OsStr::new("--target=").to_owned();
        target_str.push(target);
        args.push(&target_str);
    }

    let mut manifest_path_str;
    if let Some(mft) = &bnr.manifest_path {
        manifest_path_str = OsStr::new("--manifest-path=").to_owned();
        manifest_path_str.push(mft);
        args.push(&manifest_path_str);
    }

    let mut message_format_path_strs = Vec::new();
    for path in &bnr.message_format_path {
        let mut message_format_path_str = OsStr::new("--message-format-path=").to_owned();
        message_format_path_str.push(path);
        message_format_path_strs.push(message_format_path_str);
    }
    for message_format_path_str in &message_format_path_strs {
        args.push(message_format_path_str);
    }

    for _ in 0..bnr.verbose {
        args.push(OsStr::new("-v"));
    }

    let mut color_str;
    if let Some(color) = bnr.color {
        color_str = OsStr::new("--color=").to_owned();
        color_str.push(OsStr::new::<str>(color.into()));
        args.push(&color_str);
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
    Command::new(exe).args(&bnr.args).spawn().ok()
}

fn main() {
    let build_n_run = BuildNRun::parse();

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
            if let Some(mut child) = proc.take() {
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

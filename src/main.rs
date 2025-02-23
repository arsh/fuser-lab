use clap::*;
use fuser::MountOption;
use simple::SimpleFS;
use tracing_subscriber;

mod simple;
fn setup_logging(level: &str) {
    let level = match level {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    let default_subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(default_subscriber)
        .expect("setting tracing default failed");
}
fn main() {
    let matches =
        Command::new("simple")
            .version(crate_version!())
            .arg(Arg::new("SOURCE_DIRECTORY").required(true).index(1).help(
                "Source directory. Typically a local filesystem that actually holds the files.",
            ))
            .arg(
                Arg::new("MOUNT_POINT")
                    .required(true)
                    .index(2)
                    .help("Act as a client, and mount FUSE at given path"),
            )
            .arg(
                Arg::new("LOG_LEVEL")
                    .long("log_level")
                    .default_value("info")
                    .help("One of these: info, trace, debug, error"),
            )
            .arg(
                Arg::new("auto_unmount")
                    .long("auto_unmount")
                    .action(ArgAction::SetTrue)
                    .help("Automatically unmount on process exit"),
            )
            .arg(
                Arg::new("allow-root")
                    .long("allow-root")
                    .action(ArgAction::SetTrue)
                    .help("Allow root user to access filesystem"),
            )
            .get_matches();
    env_logger::init();
    let source_dir = matches.get_one::<String>("SOURCE_DIRECTORY").unwrap();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();

    let log_level = matches.get_one::<String>("LOG_LEVEL").unwrap();
    setup_logging(&log_level);

    let mut options = vec![MountOption::RO, MountOption::FSName("simple".to_string())];
    if matches.get_flag("auto_unmount") {
        options.push(MountOption::AutoUnmount);
    }
    if matches.get_flag("allow-root") {
        options.push(MountOption::AllowRoot);
    }

    fuser::mount2(SimpleFS::new(source_dir.to_owned()), mountpoint, &options).unwrap();
}

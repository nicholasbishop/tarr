use argh::FromArgs;
use std::path::PathBuf;

/// Tarball utility.
#[derive(Debug, FromArgs)]
struct Opt {
    #[argh(subcommand)]
    command: Command,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum Command {
    List(ListCommand),
    // TODO
    // Pack,
    // Unpack,
}

/// List the contents of a tarball.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "list")]
struct ListCommand {
    #[argh(positional)]
    tarball: PathBuf,
}

fn main() {
    let opt: Opt = argh::from_env();
    dbg!(&opt);

    match opt.command {
        Command::List(list) => {}
    }
}

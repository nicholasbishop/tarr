use anyhow::{anyhow, Error};
use argh::FromArgs;
use fehler::throws;
use humansize::{file_size_opts as options, FileSize};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;
use unicode_width::UnicodeWidthStr;

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

#[throws]
fn list_tarball_impl<R: Read, P: FnMut(&str)>(
    archive: &mut Archive<R>,
    mut print: P,
) {
    struct Entry {
        path: String,
        size: String,
    }

    let mut max_path_columns = 0;
    let mut entries = archive
        .entries()?
        .map(|file| -> Result<Entry, Error> {
            let file = file?;
            let header = file.header();
            let size = match header.size()?.file_size(options::BINARY) {
                Ok(size) => size,
                Err(err) => {
                    return Err(anyhow!(err));
                }
            };

            let path = header.path()?.display().to_string();
            let path_columns = path.width();
            if path_columns > max_path_columns {
                max_path_columns = path_columns;
            }

            Ok(Entry { path, size })
        })
        .collect::<Result<Vec<_>, _>>()?;

    entries.sort_unstable_by_key(|e| e.path.clone());

    for entry in entries {
        print(&format!(
            "{:path_width$} {}",
            entry.path,
            entry.size,
            path_width = max_path_columns
        ));
    }
}

#[throws]
fn list_tarball(list: ListCommand) {
    // TODO: decompression
    let file = File::open(list.tarball).unwrap();
    let mut archive = Archive::new(file);

    list_tarball_impl(&mut archive, |s| println!("{}", s))?;
}

#[throws]
fn main() {
    let opt: Opt = argh::from_env();

    match opt.command {
        Command::List(list) => {
            list_tarball(list)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tarball() {
        let file = include_bytes!("../tests/test.tar").to_vec();
        let mut archive = Archive::new(file.as_slice());

        let mut lines = Vec::new();
        list_tarball_impl(&mut archive, |s| lines.push(s.to_string())).unwrap();

        assert_eq!(
            lines,
            vec![
                "Cargo.lock 4.80 KiB",
                "Cargo.toml 187 B",
                "LICENSE    11.09 KiB",
            ]
        );
    }
}

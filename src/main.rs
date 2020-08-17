use anyhow::{anyhow, Error};
use argh::FromArgs;
use fehler::throws;
use humansize::{file_size_opts as options, FileSize};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
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
    Unpack(UnpackCommand),
}

/// List the contents of a tarball.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "list")]
struct ListCommand {
    #[argh(positional)]
    tarball: PathBuf,
}

/// Unpack the contents of a tarball.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "unpack")]
struct UnpackCommand {
    #[argh(positional)]
    tarball: PathBuf,
}

fn print_str(s: &str) {
    println!("{}", s);
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

    list_tarball_impl(&mut archive, print_str)?;
}

/// This is similar to Path::file_stem, but it additionally strips off
/// the ".tar" extension if that is present behind the first
/// extension.
///
/// Examples:
///
///     foo.tar.gz -> foo
///     foo.tar -> foo
///     foo -> foo
///     foo.bar.gz -> foo.bar
fn file_stem(path: &Path) -> Option<&OsStr> {
    let stem = path.file_stem()?;
    let stem_path = Path::new(stem);
    if stem_path.extension() == Some(OsStr::new("tar")) {
        stem_path.file_stem()
    } else {
        Some(stem)
    }
}

enum DirContents {
    /// Directory is empty
    Empty,
    /// Directory contains only one child (either file or directory)
    One(PathBuf),
    /// Directory contains multiple children (files or directories)
    Multiple,
}

impl DirContents {
    #[throws]
    fn new(dir: &Path) -> DirContents {
        // Check if there's more than one file in the temporary directory
        let mut first_file = None;
        let mut count = 0;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            count += 1;
            if first_file.is_none() {
                first_file = Some(entry.path());
            } else {
                break;
            }
        }

        if count == 0 {
            DirContents::Empty
        } else if count == 1 {
            // OK to unwrap: first_file has been set if count > 0
            DirContents::One(first_file.unwrap())
        } else {
            DirContents::Multiple
        }
    }
}

#[throws]
fn unpack_tarball_impl<R: Read, P: FnMut(&str)>(
    archive: &mut Archive<R>,
    source: &Path,
    destination: &Path,
    mut print: P,
) {
    // Unpack into a temporary directory
    let tmp_dir = tempfile::Builder::new().tempdir_in(&destination)?;
    archive.unpack(tmp_dir.path())?;

    // Check if there's more than one file in the temporary directory
    match DirContents::new(tmp_dir.path())? {
        DirContents::Empty => {
            print("empty tarball");
        }
        DirContents::One(path) => {
            // OK to unwrap: this path comes from a directory listing,
            // we know the path doesn't terminate in "..".
            let target_path = destination.join(path.file_name().unwrap());
            fs::rename(path, &target_path)?;
            print(&format!("unpacked to {}", target_path.display()));
        }
        DirContents::Multiple => {
            // OK to unwrap: file_stem can only return None if the input
            // path has no file component, but since we've already
            // successfully unpacked the tarball we know the path has a
            // file name.
            let new_dir = destination.join(file_stem(&source).unwrap());
            // TODO: check if the target path already exists and deal with
            // that in some way
            let tmp_path = tmp_dir.path();
            fs::rename(tmp_path, &new_dir)?;
            print(&format!("unpacked to {}", new_dir.display()));
        }
    }
}

#[throws]
fn unpack_tarball(unpack: UnpackCommand) {
    // TODO: decompression
    let file = File::open(&unpack.tarball).unwrap();
    let mut archive = Archive::new(file);

    let cwd = env::current_dir()?;

    unpack_tarball_impl(&mut archive, &unpack.tarball, &cwd, print_str)?;
}

#[throws]
fn main() {
    let opt: Opt = argh::from_env();

    match opt.command {
        Command::List(list) => {
            list_tarball(list)?;
        }
        Command::Unpack(unpack) => {
            unpack_tarball(unpack)?;
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

    #[test]
    fn test_file_stem() {
        assert_eq!(file_stem(Path::new("foo")).unwrap(), "foo");
        assert_eq!(file_stem(Path::new("foo.tar")).unwrap(), "foo");
        assert_eq!(file_stem(Path::new("foo.tar.gz")).unwrap(), "foo");
        assert_eq!(file_stem(Path::new("foo.bar.tar")).unwrap(), "foo.bar");
    }

    #[test]
    fn test_unpack_tarball() {}
}

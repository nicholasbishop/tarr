use anyhow::{anyhow, Error};
use argh::FromArgs;
use fehler::{throw, throws};
use humansize::{file_size_opts as options, FileSize};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
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
fn has_common_prefix<R: Read>(archive: &mut Archive<R>) -> bool {
    let mut prefix: Option<OsString> = None;
    for file in archive.entries()? {
        let file = file?;
        let header = file.header();
        let path = header.path()?;

        if path.is_absolute() {
            // TODO
            panic!("absolute paths not yet handled");
        }

        if let Some(Component::Normal(comp)) = path.components().next() {
            if let Some(prefix) = &prefix {
                if prefix != comp {
                    return false;
                }
            } else {
                prefix = Some(comp.into());
            }
        } else {
            // TODO
            panic!("unexpected path format in tarball");
        }
    }

    true
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

#[throws]
fn unpack_tarball(unpack: UnpackCommand) {
    // TODO: decompression
    let file = File::open(&unpack.tarball).unwrap();
    let mut archive = Archive::new(file);

    let mut unpack_dir = env::current_dir()?;

    // Check if all the files in the tarball are in a common
    // directory. If not, create one based on the name of the
    // tarball. This avoids ever accidentally bombing a directory with
    // the contents of an ill-mannered tarball.
    if !has_common_prefix(&mut archive)? {
        if let Some(stem) = file_stem(&unpack.tarball) {
            unpack_dir = unpack_dir.join(stem);
            fs::create_dir(unpack_dir)?;
        } else {
            // TODO: improve error
            throw!(anyhow!("failed to get file stem"));
        }
    }
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
}

# tarr

**This tool is no longer under active development. If you are interested in taking over or repurposing the name on crates.io, feel free to contact me: nbishop@nbishop.net**

Command-line tarball utility written in Rust.

This tool is a work in progress. Currently it supports two operations:
list and unpack. If the tarball contains more than one file not in a
common directory, the files are unpacked into a new directory with a
name based on the tarball. This ensures that an ill-mannered tarball
cannot bomb the output directory.

## TODO

- Pack command(s): this command will create a tarball. The interface
  here needs a bit of thought since you need to control both the file
  inputs and their paths within the tarball. With `tar` you typically
  do this by adding a `-C`, not sure if that's the most convenient way
  forward yet though.
  
- Automatic decompression -- the unpack command should be able to
  handle gz, xz, bz2, lz4, etc.
  
- Automatic compression -- same for the pack command.

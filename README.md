# tarr

Command-line tarball utility written in Rust.

This tool is a work in progress. Currently it supports two operations:
list and unpack. If the tarball contains more than one file not in a
common directory, the files are unpacked into a new directory with a
name based on the tarball. This ensures that an ill-mannered tarball
cannot bomb the output directory.

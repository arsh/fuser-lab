# Simple FS

Repo that contains a very simple FS based on [FUSER](https://github.com/cberner/fuser). It only supports limited amount of operations for a read-only filesystem that is actually backed by an existing (typically EXT4) source. Essentially just a proxy to test the overhead and performance implications of the FUSE layer.

This is based on modified code from the [hello.rs](https://github.com/cberner/fuser/blob/master/examples/hello.rs) example from the project [FUSER](https://github.com/cberner/fuser)
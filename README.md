# Simple FS

Repo that contains a very simple FS (read-only, no sub directories) based on [FUSER](https://github.com/cberner/fuser). It only supports limited amount of operations for a read-only filesystem that is actually backed by an existing (typically EXT4) source. Essentially just a proxy to test the overhead and performance implications of the FUSE layer.

This is based on modified code from the [hello.rs](https://github.com/cberner/fuser/blob/master/examples/hello.rs) example from the project [FUSER](https://github.com/cberner/fuser)

## To run / test it
Note that this only runs in foreground.

```
# ~/ext4-source is the source fs to read from
# ~/mnt is where the fs will be mount it

### RUN ###
# with cargo
cargo run -- --auto_unmount ~/ext4-source ~/mnt

# or by itself
./fuser-lab --auto_unmount ~/ext4-source ~/mnt

### TEST ###
# on a separate terminal
~ » ls ~/mnt
hello.txt  ten_gb.bin  world.txt

~ » cat ~/mnt/hello.txt
hello from ext4

```

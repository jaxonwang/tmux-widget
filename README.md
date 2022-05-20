# Tmux Widgets in Rust

### Build

```
cargo build --release
```

### Usage

Print network bandwidth
```
./target/release/tmux-widget --net --with-icons --interval 1
```

Print CPU and memory usage
```
./target/release/tmux-widget --cpu-mem --with-icons
```

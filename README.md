# Tmux Widgets in Rust

### Build

```
cargo build --release
```

### Usage

Print network bandwidth
```
tmux-widget --net --with-icons --interval=1
```

Print CPU and memory usage
```
tmux-widget --cpu-mem --with-icons
```

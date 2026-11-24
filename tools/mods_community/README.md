# mods_community

Checks every mod in `mods/` (or `--only <id>`) against the README's rules. It reads, and runs nothing of a mod.

```
cargo run --release -- check --root ../..
cargo run --release -- pack --root ../.. --out <an empty folder>
cargo test
```

The tests check the mods of this repository too.

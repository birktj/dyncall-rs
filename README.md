# dyncall-rs

## Example
```rust
use dyncall::DyncallLib;

fn main() {
    let lib = DyncallLib::new("libc.so.6");
    
    let mut fun = lib.func(b"abs");
    fun.add_arg::<i32>(-5);

    let res = unsafe { fun.call::<i32>() };

    assert_eq!(res, 5);
}
```

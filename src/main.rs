fn main() {
    let lib = dyncall::DyncallLib::new("libc.so.6");
    
    let mut fun = lib.func(b"abs");
    fun.add_arg::<i32>(-5);

    let res = unsafe { fun.call::<i32>() };

    println!("Res: {:?}", res);
}

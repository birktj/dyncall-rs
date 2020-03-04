use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use cranelift_simplejit::{SimpleJITBackend, SimpleJITBuilder};
use target_lexicon::Triple;
use libloading::{Library, Symbol};

use std::mem;

pub struct DyncallLib {
    lib: Library
}

impl DyncallLib {
    pub fn new(libname: &str) -> DyncallLib {
        DyncallLib {
            lib: Library::new(libname).unwrap()
        }
    }

    pub fn func(&self, name: &[u8]) -> DyncallFunc {
        let sym: Symbol<*const u8> = unsafe { self.lib.get(name).unwrap() };

        DyncallFunc {
            addr: *sym,
            args: Vec::new(),
        }
    }
}

pub enum ArgValue {
    I8(u8),
    I16(u16),
    I32(u32),
    I64(u64),
    B(bool),
    Ptr(*mut ())
}

impl ArgValue {
    fn ty(&self) -> Type {
        match self {
            ArgValue::I8(_) => types::I8,
            ArgValue::I16(_) => types::I16,
            ArgValue::I32(_) => types::I32,
            ArgValue::I64(_) => types::I64,
            ArgValue::B(_) => types::B8,
            ArgValue::Ptr(_) => types::I64, // FIXME
        }
    }
}
impl From<u8> for ArgValue {
    fn from(value: u8) -> Self {
        ArgValue::I8(value)
    }
}

impl From<i8> for ArgValue {
    fn from(value: i8) -> Self {
        ArgValue::I8(value as u8)
    }
}

impl From<u16> for ArgValue {
    fn from(value: u16) -> Self {
        ArgValue::I16(value)
    }
}

impl From<i16> for ArgValue {
    fn from(value: i16) -> Self {
        ArgValue::I16(value as u16)
    }
}

impl From<u32> for ArgValue {
    fn from(value: u32) -> Self {
        ArgValue::I32(value)
    }
}

impl From<i32> for ArgValue {
    fn from(value: i32) -> Self {
        ArgValue::I32(value as u32)
    }
}

impl From<u64> for ArgValue {
    fn from(value: u64) -> Self {
        ArgValue::I64(value)
    }
}

impl From<i64> for ArgValue {
    fn from(value: i64) -> Self {
        ArgValue::I64(value as u64)
    }
}

impl From<bool> for ArgValue {
    fn from(value: bool) -> Self {
        ArgValue::B(value)
    }
}

impl<T> From<&T> for ArgValue {
    fn from(value: &T) -> Self {
        ArgValue::Ptr(value as *const T as *mut ())
    }
}

pub trait ValueType {
    fn value_type() -> Option<Type>;
}

impl ValueType for () {
    fn value_type() -> Option<Type> {
        None
    }
}

impl ValueType for u8 {
    fn value_type() -> Option<Type> {
        Some(types::I8)
    }
}

impl ValueType for i8 {
    fn value_type() -> Option<Type> {
        Some(types::I8)
    }
}

impl ValueType for u16 {
    fn value_type() -> Option<Type> {
        Some(types::I16)
    }
}

impl ValueType for i16 {
    fn value_type() -> Option<Type> {
        Some(types::I16)
    }
}

impl ValueType for u32 {
    fn value_type() -> Option<Type> {
        Some(types::I32)
    }
}

impl ValueType for i32 {
    fn value_type() -> Option<Type> {
        Some(types::I32)
    }
}

impl ValueType for u64 {
    fn value_type() -> Option<Type> {
        Some(types::I64)
    }
}

impl ValueType for i64 {
    fn value_type() -> Option<Type> {
        Some(types::I64)
    }
}

pub struct DyncallFunc {
    addr: *const u8,
    args: Vec<ArgValue>
}

impl DyncallFunc {
    pub fn add_arg<T: Into<ArgValue>>(&mut self, arg: T) -> &mut Self {
        self.args.push(arg.into());
        self
    }

    fn get_call_ptr(&self, rettype: Option<Type>) -> *const u8 {
        let mut module: Module<SimpleJITBackend> = {
            let mut jit_builder = SimpleJITBuilder::new(cranelift_module::default_libcall_names());
            jit_builder.symbol("extern", self.addr);
            Module::new(jit_builder)
        };

        let mut ctx = module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut sig_func = module.make_signature();
        sig_func.call_conv = isa::CallConv::triple_default(&Triple::host());
        for arg in &self.args {
            sig_func.params.push(AbiParam::new(arg.ty()))
        }
        if let Some(ty) = rettype {
            sig_func.returns.push(AbiParam::new(ty));
        }

        let func_extern = module
            .declare_function("extern", Linkage::Import, &sig_func)
            .unwrap();

        let mut sig_call = module.make_signature();
        if let Some(ty) = rettype {
            sig_call.returns.push(AbiParam::new(ty));
        }

        let func_call = module
            .declare_function("call", Linkage::Local, &sig_call)
            .unwrap();

        ctx.func.signature = sig_call;
        ctx.func.name = ExternalName::user(0, func_call.as_u32());
        {
            let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            let ebb = bcx.create_block();

            bcx.switch_to_block(ebb);
            let local_func = module.declare_func_in_func(func_extern, &mut bcx.func);
            let mut args = Vec::new();
            for arg in &self.args {
                match arg {
                    ArgValue::I8(x) => args.push(bcx.ins().iconst(arg.ty(), *x as i64)),
                    ArgValue::I16(x) => args.push(bcx.ins().iconst(arg.ty(), *x as i64)),
                    ArgValue::I32(x) => args.push(bcx.ins().iconst(arg.ty(), *x as i64)),
                    ArgValue::I64(x) => args.push(bcx.ins().iconst(arg.ty(), *x as i64)),
                    ArgValue::B(x) => args.push(bcx.ins().bconst(arg.ty(), *x)),
                    ArgValue::Ptr(ptr) => args.push(bcx.ins().iconst(arg.ty(), *ptr as i64)),
                }
            }

            let call = bcx.ins().call(local_func, &args);

            match rettype {
                Some(_) => {
                    let results = bcx.inst_results(call);
                    assert_eq!(results.len(), 1);
                    let results = results.to_owned();
                    bcx.ins().return_(&results);
                }
                None => {
                    bcx.ins().return_(&[]);
                }
            }

            bcx.seal_all_blocks();
            bcx.finalize();
        }

        module.define_function(func_call, &mut ctx).unwrap();
        module.clear_context(&mut ctx);

        // Perform linking.
        module.finalize_definitions();

        // Get a raw pointer to the generated code.
        module.get_finalized_function(func_call)
    }

    pub unsafe fn call<T: ValueType>(self) -> T {
        let ptr = self.get_call_ptr(T::value_type());
        
        let fun = mem::transmute::<_, fn() -> T>(ptr);

        (fun)()
    }
}

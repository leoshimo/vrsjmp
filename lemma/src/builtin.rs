//! Builtin func
use crate::{Error, Fiber, FiberState, NativeFn, NativeFnVal, SymbolId, Val};

/// Native binding for `+`
pub fn plus_fn() -> NativeFn {
    // TODO: Write tests for +
    // TODO: Support N operands
    NativeFn {
        symbol: SymbolId::from("+"),
        func: |_, x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(NativeFnVal::Return(Val::Int(a + b))),
            _ => panic!("only supports ints"),
        },
    }
}

/// Native binding for `peval`
pub fn peval_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("peval"),
        func: |f, args| {
            let v = match args {
                [v] => v,
                _ => {
                    return Err(Error::InvalidExpression(
                        "peval expects one argument".to_string(),
                    ))
                }
            };
            // TODO: Hack - FWIW the yield here only works if we never implement nonlocal returns
            let mut f = Fiber::from_val(v)?.with_env(f.env());
            match f.resume() {
                Ok(FiberState::Done(v)) => Ok(NativeFnVal::Return(v)),
                Ok(FiberState::Yield(v)) => Ok(NativeFnVal::Yield(v)),
                Err(e) => Ok(NativeFnVal::Return(Val::Error(e))),
            }
        },
    }
}

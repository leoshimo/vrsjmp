//! Tests for embedding in host application

use assert_matches::assert_matches;
use lyric::{parse, Error, FiberState, Inst, NativeFn, NativeFnVal, SymbolId};
use FiberState::*;

type Fiber = lyric::Fiber<Ext, Locals>;
type Val = lyric::Val<Ext, Locals>;

#[derive(Debug, Clone, PartialEq)]
enum Ext {
    SendConn(Vec<Val>),
    RecvConn,
    EchoYield(Vec<Val>),
}

#[derive(Debug, Clone, PartialEq)]
struct Locals {
    val: i32,
}

/// Helper - create local fixture
fn locals() -> Locals {
    Locals { val: 0 }
}

impl std::fmt::Display for Ext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Ext {:?}>", self)
    }
}

#[test]
fn fiber_simple() {
    let mut f = Fiber::from_expr("\"hello world\"", locals()).unwrap();
    assert_eq!(f.resume(), Ok(Done(Val::string("hello world"))));
}

#[test]
fn fiber_invalid_expr() {
    assert_matches!(
        Fiber::from_expr("- jibberish )(", locals()),
        Err(Error::FailedToLex(_))
    );
}

#[test]
fn fiber_empty_bytecode() {
    let mut f = Fiber::from_bytecode(vec![], locals());
    assert_matches!(
        f.resume(),
        Err(Error::UnexpectedStack(_)),
        "Executing empty bytecode panics, since there is nothing to return"
    );
}

#[test]
fn fiber_invalid_bytecode() {
    let mut f = Fiber::from_bytecode(vec![Inst::PopTop, Inst::PopTop, Inst::PopTop], locals());
    assert_matches!(f.resume(), Err(Error::UnexpectedStack(_)));
}

#[test]
fn fiber_yielding() {
    // An infinitely increasing counter increasing by one each iteration
    let prog = r#"
        (begin
            (def x 0)
            (defn yielding_add ()
                (yield x)
                (set x (+ x 1))
                (yielding_add))
            (yielding_add)
        )
    "#;
    let mut f = Fiber::from_expr(prog, locals()).unwrap();

    assert_eq!(f.resume().unwrap(), Yield(Val::Int(0)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(1)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(2)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(3)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(4)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(5)));

    // TODO: Use fiber_yielding as test case for tail-call optimization (shouldn't grow number of CallFrames)
}

#[test]
fn fiber_yielding_by_arg() {
    // An infinitely increasing counter increasing by yield-ed value
    let prog = r#"
        (begin
            (def x 0)
            (defn yielding_add ()
                (set x (+ x (yield x)))
                (yielding_add))
            (yielding_add)
        )
    "#;
    let mut f = Fiber::from_expr(prog, locals()).unwrap();

    assert_eq!(f.resume().unwrap(), Yield(Val::Int(0)));
    assert_eq!(
        f.resume_from_yield(Val::Int(1)).unwrap(),
        Yield(Val::Int(1))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(2)).unwrap(),
        Yield(Val::Int(3))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(3)).unwrap(),
        Yield(Val::Int(6))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(4)).unwrap(),
        Yield(Val::Int(10))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(5)).unwrap(),
        Yield(Val::Int(15))
    );
}

#[test]
fn fiber_yielding_native_binding() {
    let mut f = Fiber::from_expr("(echo_yield :one :two)", locals()).unwrap();
    f.bind(NativeFn {
        symbol: SymbolId::from("echo_yield"),
        func: |_, x| Ok(NativeFnVal::Yield(Val::Extern(Ext::EchoYield(x.to_vec())))),
    });

    assert_eq!(
        f.resume().unwrap(),
        Yield(Val::Extern(Ext::EchoYield(vec![
            Val::keyword("one"),
            Val::keyword("two"),
        ])))
    );
    assert_eq!(
        f.resume_from_yield(Val::string("Hello world")).unwrap(),
        Done(Val::string("Hello world")),
    );
}

#[test]
fn fiber_looping_yield() {
    let prog = r#"
        (begin
            (def x 0)
            (loop (set x (+ x (yield x)))))
    "#;

    let mut f = Fiber::from_expr(prog, locals()).unwrap();

    assert_eq!(f.resume().unwrap(), Yield(Val::Int(0)));
    assert_eq!(
        f.resume_from_yield(Val::Int(1)).unwrap(),
        Yield(Val::Int(1))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(2)).unwrap(),
        Yield(Val::Int(3))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(3)).unwrap(),
        Yield(Val::Int(6))
    );
}

#[test]
fn fiber_conn_recv_peval_sim() {
    // program representing client REPL loop
    let prog = r#"
        (loop (send_conn (peval (recv_conn))))
    "#;

    let mut f = Fiber::from_expr(prog, locals()).unwrap();
    f.bind(NativeFn {
        symbol: SymbolId::from("recv_conn"),
        func: |_, _| Ok(NativeFnVal::Yield(Val::Extern(Ext::RecvConn))),
    });
    f.bind(NativeFn {
        symbol: SymbolId::from("send_conn"),
        func: |_, args| {
            Ok(NativeFnVal::Yield(Val::Extern(Ext::SendConn(
                args.to_vec(),
            ))))
        },
    });

    assert_eq!(f.resume().unwrap(), Yield(Val::Extern(Ext::RecvConn)));

    assert_eq!(
        f.resume_from_yield(parse("(def x (+ 1 2))").unwrap().into())
            .unwrap(),
        Yield(Val::Extern(Ext::SendConn(vec![Val::Int(3)]))),
        "Should receive send_conn signal w/ eval-ed expr"
    );
    assert_eq!(
        f.resume_from_yield(Val::Nil).unwrap(),
        Yield(Val::Extern(Ext::RecvConn))
    );

    assert_eq!(
        f.resume_from_yield(parse("x").unwrap().into()).unwrap(),
        Yield(Val::Extern(Ext::SendConn(vec![Val::Int(3)]))),
    );
    assert_eq!(
        f.resume_from_yield(Val::Nil).unwrap(),
        Yield(Val::Extern(Ext::RecvConn))
    );

    assert_eq!(
        f.resume_from_yield(Val::symbol("jibberish")).unwrap(),
        Yield(Val::Extern(Ext::SendConn(vec![Val::Error(
            Error::UndefinedSymbol(SymbolId::from("jibberish"))
        )]))),
        "Error should return error as a value via pcall"
    );
    assert_eq!(
        f.resume_from_yield(Val::Nil).unwrap(),
        Yield(Val::Extern(Ext::RecvConn))
    );

    assert_eq!(
        f.resume_from_yield(parse("(set x (+ x x))").unwrap().into())
            .unwrap(),
        Yield(Val::Extern(Ext::SendConn(vec![Val::Int(6)]))),
        "Environment should be preserved after error"
    );
}

#[test]
fn get_set_locals() {
    let prog = r#"
        (begin
            (inc_local (+ 9 (get_local)))
            (inc_local 1)
            (inc_local 2)
            (get_local))"#;
    let mut f = Fiber::from_expr(prog, Locals { val: 15 }).unwrap();
    f.bind(NativeFn {
        symbol: SymbolId::from("get_local"),
        func: |f, _| {
            let v = f.locals().val;
            Ok(NativeFnVal::Return(Val::Int(v)))
        },
    });
    f.bind(NativeFn {
        symbol: SymbolId::from("inc_local"),
        func: |f, args| {
            let v = match args {
                [Val::Int(v)] => v,
                _ => panic!(),
            };
            f.locals_mut().val += v;
            Ok(NativeFnVal::Return(Val::Nil))
        },
    });

    assert_eq!(f.resume().unwrap(), FiberState::Done(Val::Int(42)));
}

//! A fiber of execution that can be cooperatively scheduled via yielding.
use tracing::debug;

use super::{Env, Inst};
use crate::{Form, Lambda, SymbolId};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct Fiber {
    cframes: Vec<CallFrame>,
    stack: Vec<Form>,
    status: Status,
}

/// Status for a fiber
#[derive(Debug, PartialEq)]
pub enum Status {
    New,
    Running,
    Yielded,
    Completed(Result<Form>),
}

/// Single call frame
#[derive(Debug)]
struct CallFrame {
    ip: usize,
    code: Vec<Inst>,
    env: Rc<RefCell<Env>>,
}

/// Errors encountered during execution of expression
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum FiberError {
    /// A fiber that is already running was asked to start
    #[error("Fiber is already running")]
    AlreadyRunning,

    /// Setting undefined symbol
    #[error("Undefined symbol - {0}")]
    UndefinedSymbol(SymbolId),

    /// Unexpected state on stack
    #[error("Unexpected stack state - {0}")]
    UnexpectedStack(String),
}

/// Result type for fiber ops
pub type Result<T> = std::result::Result<T, FiberError>;

/// Start execution of a fiber
pub fn start(f: &mut Fiber) -> Result<()> {
    if f.status != Status::New {
        return Err(FiberError::AlreadyRunning);
    }
    run(f);
    Ok(())
}

/// Run the fiber until it completes or yields
fn run(f: &mut Fiber) {
    f.status = Status::Running;

    f.status = loop {
        let inst = f.inst().clone(); // TODO(opt): Defer cloning until args need cloning
        f.top_mut().ip += 1;

        // TODO: Add fiber debug flag
        debug!(
            "frame {} ip {}: {:?}",
            f.cframes.len() - 1,
            f.top().ip,
            inst
        );

        match inst {
            Inst::PushConst(form) => {
                f.stack.push(form);
            }
            Inst::StoreSym(s) => match f.stack.last() {
                Some(value) => f.top().env.borrow_mut().define(&s, value.clone()),
                None => {
                    break Status::Completed(Err(FiberError::UnexpectedStack(
                        "Expected stack to be nonempty".to_string(),
                    )))
                }
            },
            Inst::LoadSym(s) => match f.top().env.clone().borrow().get(&s) {
                Some(value) => f.stack.push(value.clone()),
                None => {
                    break Status::Completed(Err(FiberError::UndefinedSymbol(s)));
                }
            },
            Inst::MakeFunc => {
                // TODO(ref): Break makes error prop verbose
                let code = match f.stack.pop() {
                    Some(Form::Bytecode(b)) => b,
                    _ => {
                        break Status::Completed(Err(FiberError::UnexpectedStack(
                            "Missing function bytecode".to_string(),
                        )))
                    }
                };
                let params = match f.stack.pop() {
                    Some(Form::List(p)) => p,
                    _ => {
                        break Status::Completed(Err(FiberError::UnexpectedStack(
                            "Missing parameter list".to_string(),
                        )))
                    }
                };
                let params = params
                    .into_iter()
                    .map(|f| match f {
                        Form::Symbol(s) => Ok(s),
                        _ => Err(FiberError::UnexpectedStack(
                            "Unexpected parameter list".to_string(),
                        )),
                    })
                    .collect::<Result<Vec<_>>>();
                let params = match params {
                    Ok(p) => p,
                    Err(e) => break Status::Completed(Err(e)),
                };

                f.stack.push(Form::Lambda(Lambda { params, code }));
            }
            Inst::CallFunc(nargs) => {
                let args = (0..nargs)
                    .map(|_| {
                        f.stack.pop().ok_or(FiberError::UnexpectedStack(
                            "Missing expected {nargs} args".to_string(),
                        ))
                    })
                    .rev()
                    .collect::<Result<Vec<_>>>();
                let args = match args {
                    Ok(a) => a,
                    Err(e) => break Status::Completed(Err(e)),
                };
                let lambda = match f.stack.pop() {
                    Some(Form::Lambda(l)) => l,
                    _ => {
                        break Status::Completed(Err(FiberError::UnexpectedStack(
                            "Missing function object".to_string(),
                        )))
                    }
                };

                let mut fn_env = Env::extend(&f.top().env); // TODO: should use parent scope!
                for (s, arg) in lambda.params.iter().zip(args) {
                    fn_env.define(s, arg);
                }

                f.cframes
                    .push(CallFrame::from_bytecode(fn_env, lambda.code));
            }
            Inst::PopTop => todo!(),
            Inst::BeginScope => todo!(),
            Inst::EndScope => todo!(),
        }

        // end of func
        if f.status == Status::Running {
            while f.cframes.len() > 1 && f.top().is_done() {
                f.cframes.pop();
            }
            if f.at_end() {
                let res = f.stack.pop().expect("Stack should contain result");
                break Status::Completed(Ok(res));
            }
        }
    };
}

impl Fiber {
    /// Create a new fiber from given bytecode
    pub fn from_bytecode(bytecode: Vec<Inst>) -> Self {
        Fiber {
            stack: vec![],
            cframes: vec![CallFrame::from_bytecode(Env::default(), bytecode)],
            status: Status::New,
        }
    }

    /// Reference to top of callstack
    fn top(&self) -> &CallFrame {
        self.cframes.last().expect("Fiber has no callframes!")
    }

    /// Reference to top of callstack
    fn top_mut(&mut self) -> &mut CallFrame {
        self.cframes.last_mut().expect("Fiber has no callframes!")
    }

    /// Get the current instruction
    fn inst(&self) -> &Inst {
        let top = self.top();
        let inst = top
            .code
            .get(top.ip)
            .expect("Callstack executing outside bytecode length");
        inst
    }

    /// Exhausted all call frames
    fn at_end(&self) -> bool {
        self.cframes.len() == 1 && self.cframes.last().unwrap().is_done()
    }
}

impl CallFrame {
    /// Create a new callframe for executing given bytecode from start
    pub fn from_bytecode(env: Env, code: Vec<Inst>) -> Self {
        Self {
            ip: 0,
            env: Rc::new(RefCell::new(env)),
            code,
        }
    }

    /// Whether or not call frame should return (i.e. on last call frame execution)
    pub fn is_done(&self) -> bool {
        self.ip == self.code.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Inst::*;
    use super::*;
    use assert_matches::assert_matches;
    use tracing_test::traced_test;

    #[test]
    fn fiber_load_const_return() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Form::Int(5))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Form::Int(5))));

        let mut f = Fiber::from_bytecode(vec![PushConst(Form::string("Hi"))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Form::string("Hi"))));
    }

    #[test]
    #[traced_test]
    fn store_symbol() {
        // fiber for (def x 5)
        let mut f =
            Fiber::from_bytecode(vec![PushConst(Form::Int(5)), StoreSym(SymbolId::from("x"))]);

        start(&mut f).expect("should start");

        assert_eq!(
            f.top().env.borrow().get(&SymbolId::from("x")),
            Some(Form::Int(5)),
            "symbol should be defined in env"
        );
        assert_eq!(
            f.status,
            Status::Completed(Ok(Form::Int(5))),
            "should complete with value of stored symbol"
        );
    }

    #[test]
    fn load_symbol() {
        let mut f = Fiber::from_bytecode(vec![LoadSym(SymbolId::from("x"))]);
        f.top()
            .env
            .borrow_mut()
            .define(&SymbolId::from("x"), Form::string("hi"));

        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Form::string("hi"))));
    }

    #[test]
    fn load_symbol_undefined() {
        let mut f = Fiber::from_bytecode(vec![LoadSym(SymbolId::from("x"))]);

        start(&mut f).unwrap();
        assert_matches!(
            f.status,
            Status::Completed(Err(FiberError::UndefinedSymbol(_)))
        );
    }

    #[test]
    fn make_func() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Form::List(vec![Form::symbol("x")])),
            PushConst(Form::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
            MakeFunc,
        ]);

        start(&mut f).unwrap();

        assert_eq!(
            f.status,
            Status::Completed(Ok(Form::Lambda(Lambda {
                params: vec![SymbolId::from("x")],
                code: vec![LoadSym(SymbolId::from("x"))],
            }))),
            "A function object was created"
        );
    }

    #[test]
    #[traced_test]
    fn call_func_bare() {
        // ((lambda () "hello"))
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Form::Lambda(Lambda {
                params: vec![],
                code: vec![PushConst(Form::string("hello"))],
            })),
            CallFunc(0),
        ]);

        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Form::string("hello"))));
    }

    #[test]
    #[traced_test]
    fn call_func_direct() {
        // ((lambda (x) x) "hello")
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Form::Lambda(Lambda {
                params: vec![SymbolId::from("x")],
                code: vec![LoadSym(SymbolId::from("x"))],
            })),
            PushConst(Form::string("hello")),
            CallFunc(1),
        ]);

        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Form::string("hello"))));
    }

    // TODO: Store / Load symbol w/ lexical scopes
}

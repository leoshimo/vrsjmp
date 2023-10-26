//! Bindings for Process Fibers
use super::proc::{Extern, NativeFn, NativeFnVal, Val};
use super::proc_io::IOCmd;
use super::ProcessId;
use lyric::{Error, Pattern, Result, SymbolId};

/// Binding for `recv_req` to receive requests over client connection
pub(crate) fn recv_req_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("recv_req"),
        func: |_, _| {
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RecvRequest,
            )))))
        },
    }
}

/// Binding for `send_resp` to send responses over client connection
pub(crate) fn send_resp_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("send_resp"),
        func: |_, args| -> std::result::Result<NativeFnVal, Error> {
            let val = match args {
                [v] => v.clone(),
                _ => {
                    return Err(Error::InvalidExpression(
                        "send_conn expects two arguments".to_string(),
                    ))
                }
            };
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::SendRequest(val),
            )))))
        },
    }
}

/// binding to create a new PID
pub(crate) fn pid_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("pid"),
        func: |_, args| {
            let pid = match args {
                [Val::Int(pid)] => pid,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "pid expects single integer argument".to_string(),
                    ))
                }
            };
            Ok(NativeFnVal::Return(Val::Extern(Extern::ProcessId(
                ProcessId::from(*pid as usize),
            ))))
        },
    }
}

/// binding to get current process's pid
pub(crate) fn self_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("self"),
        func: |f, _| {
            let pid = f.locals().pid;
            Ok(NativeFnVal::Return(Val::Extern(Extern::ProcessId(pid))))
        },
    }
}

/// Binding to list processes
pub(crate) fn ps_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("ps"),
        func: |_, _| {
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::ListProcesses,
            )))))
        },
    }
}

/// Binding to kill process
pub(crate) fn kill_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("kill"),
        func: |_, args| {
            let pid = match args {
                [Val::Extern(Extern::ProcessId(pid))] => *pid,
                [Val::Int(pid)] => ProcessId::from(*pid as usize),
                _ => {
                    return Err(Error::InvalidExpression(
                        "kill should have one integer argument".to_string(),
                    ))
                }
            };

            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::KillProcess(pid),
            )))))
        },
    }
}

/// Binding to send messages
pub(crate) fn send_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("send"),
        func: |_, args| {
            let (dst, msg) = match args {
                [Val::Extern(Extern::ProcessId(dst)), msg] => (dst, msg),
                _ => {
                    return Err(Error::InvalidExpression(
                        "Unexpected send call - (send DEST_PID DATA)".to_string(),
                    ))
                }
            };
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::SendMessage(*dst, msg.clone()),
            )))))
        },
    }
}

/// Binding to recv messages
pub(crate) fn recv_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("recv"),
        func: |_, args| {
            let pattern = match args {
                [pat] => Some(Pattern::from_val(pat.clone())),
                [] => None,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "recv expects one or no arguments - (recv [PATTERN])".to_string(),
                    ))
                }
            };
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Recv(pattern),
            )))))
        },
    }
}

/// Binding to list messages
pub(crate) fn ls_msgs_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("ls-msgs"),
        func: |_, args| {
            if !args.is_empty() {
                return Err(Error::InvalidExpression(
                    "Unexpected ls-msgs call - No arguments expected".to_string(),
                ));
            }
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::ListMessages,
            )))))
        },
    }
}

/// Binding for exec
pub(crate) fn exec_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("exec"),
        func: |_, args| {
            let (prog, args) = args.split_first().ok_or(Error::UnexpectedArguments(
                " Unexpected arguments to exec = (exec PROG [ARGS...])".to_string(),
            ))?;

            let prog = match prog {
                Val::String(s) => s.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "Expected string as first argument".to_string(),
                    ))
                }
            };

            let args = args
                .iter()
                .map(|a| match a {
                    Val::String(s) => Ok(s.clone()),
                    _ => Err(Error::UnexpectedArguments(
                        "exec can handle string arguments only".to_string(),
                    )),
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Exec(prog, args),
            )))))
        },
    }
}

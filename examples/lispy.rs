use std::collections::HashMap;
use std::rc::Rc;

use parser::Parser;

use lego::ffi::Function;
use lego::prelude::*;

type Goal = Program;
type Program = Vec<Sexp>;
#[derive(Debug, Clone)]
enum Sexp {
    List(List),
    Atom(Atom),
}
impl Sexp {
    fn as_ident(&self) -> Option<&str> {
        match self {
            Self::Atom(Atom::Ident(s)) => Some(s),
            _ => None,
        }
    }
}
type List = Vec<Sexp>;
type ListItems = Vec<Sexp>;
type Atoms = Vec<Atom>;
#[derive(Debug, Clone)]
enum Atom {
    IntLit(u64),
    Ident(String),
    ArrayLit(Vec<Sexp>),
}

paresse::grammar! {
    #![config(parser_flavor = lalr1)]
    Goal = <p:Program> => p;
    Program = {
        <s:Sexp> => vec![s],
        <mut p:Program> <s:Sexp> => {
            p.push(s);
            p
        },
    };
    Sexp = {
        <a:Atom> => Sexp::Atom(a),
        <l:List> => Sexp::List(l),
    };
    List = "\\(" <l:ListItems> "\\)" => l;
    ListItems = {
        <mut l:ListItems> <s:Sexp> => {
            l.push(s);
            l
        },
        "" => vec![],
    };
    Atom = {
        "\\[" <a:ListItems> "\\]" => Atom::ArrayLit(a),
        <n:"[0-9]+"> => Atom::IntLit(n.parse().unwrap()),
        <id:"[a-zA-Z\\+=\\-][a-zA-Z0-9_]*"> => Atom::Ident(id.to_string()),
    };
}

#[derive(Clone)]
enum Value {
    Int(Val<u64>),
    Bool(Val<bool>),
    IntArray(Rc<Proxy<Vec<u64>>>),
    Null,
}

enum Type {
    Int,
    Bool,
    Null,
}

trait AsType {
    fn unwrap_val(val: Value) -> Val<Self>
    where
        Self: Sized;
    fn into_value(val: Val<Self>) -> Value
    where
        Self: Sized;
}

impl AsType for u64 {
    fn unwrap_val(val: Value) -> Val<Self> {
        match val {
            Value::Int(val) => val,
            _ => unreachable!(),
        }
    }

    fn into_value(val: Val<Self>) -> Value {
        Value::Int(val)
    }
}

impl Value {
    fn add(self, other: Value) -> Value {
        match (self, other) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
            _ => unreachable!("invalid add"),
        }
    }

    fn sub(self, other: Value) -> Value { match (self, other) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs - rhs),
            _ => unreachable!("invalid add"),
        }
    }

    fn eq(self, other: Value) -> Value {
        match (self, other) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs.eq(&rhs)),
            _ => unreachable!("invalid add"),
        }
    }

    fn into_bool(self) -> Val<bool> {
        match self {
            Value::Bool(val) => val,
            _ => panic!(),
        }
    }
}


// TODO: if cflow sucks

fn eval_if(list: &[Sexp], env: &mut Env) -> Value {
    eval_expression.fn_call((&list[0], env))
        .into_bool()
        .then(|| {
            eval_expression(&list[1], env);
            ((), || {
                eval_expression(&list[2], env);
            })
        });

    Value::Null
}

fn fold_list(op: impl Fn(Value, Value) -> Value, s: &[Sexp], env: &mut Env) -> Value {
    let mut lhs = eval_expression(&s[0], env);
    for exp in &s[1..] {
        let rhs = eval_expression(exp, env);
        lhs = op(lhs, rhs);
    }

    lhs
}

struct Env {
    vars: HashMap<String, Value>,
}

struct Lambda {
    params: Vec<String>,
    body: Sexp,
}

fn parse_lambda(s: &Sexp) -> Lambda {
    match s {
        Sexp::List(list) => {
            match &list[0] {
                Sexp::Atom(Atom::Ident(i)) if i == "lambda"=> {
                    let params = match &list[1] {
                        Sexp::Atom(Atom::ArrayLit(list)) if list.iter().all(|it| it.as_ident().is_some()) => {
                            list.iter().map(|it| it.as_ident().unwrap().to_string()).collect()
                        },
                        _ => panic!(),
                    };

                    Lambda {
                        params,
                        body: list[2].clone(),
                    }
                },
                _ => panic!(),
            }
        },
        Sexp::Atom(atom) => todo!(),
    }
}

fn eval_foreach(s: &[Sexp], env: &mut Env) -> Value {
    let arr = eval_expression(&s[1], env);
    let lambda = parse_lambda(&s[0]);
    match arr {
        Value::IntArray(slice) => {
            let len = slice.len();
            let mut i = Var::new(0usize);
            let s = slice.as_slice();
            lego::prelude::do_while::<()>(|__ctx__| {
                while __ctx__.cond(|| i.neq(&len)) {
                    {
                        let it = Value::Int(s.get(i).deref());
                        assert_eq!(lambda.params.len(), 1);
                        let tmp = env.vars.insert(lambda.params[0].clone(), it);

                        eval_expression(&lambda.body, env);

                        if let Some(((name,  _), tmp)) = env.vars.remove_entry(&lambda.params[0]).zip(tmp) {
                            env.vars.insert(name, tmp);
                        }
                        i += 1usize;
                    }
                }
                lego::prelude::ControlFlow::Break(())
            });

            Value::Null
        }
        _ => panic!("not an array"),
    }
}

fn eval_expression(s: &Sexp, env: &mut Env) -> Value {
    match s {
        Sexp::List(list) => match &list[0] {
            Sexp::List(_vec) => todo!(),
            Sexp::Atom(Atom::Ident(s)) => match s.as_str() {
                "+" => fold_list(|a, b| a.add(b), &list[1..], env),
                "-" => fold_list(|a, b| a.sub(b), &list[1..], env),
                "=" => {
                    let lhs = eval_expression(&list[1], env);
                    let rhs = eval_expression(&list[2], env);
                    lhs.eq(rhs)
                },
                // "if" => eval_if::<u64>(list, env),
                "foreach" => eval_foreach(&list[1..], env),
                "print" => {
                    let val = eval_expression(&list[1], env);
                    print(val);
                    Value::Null
                },
                "if" => {
                    eval_if(&list[1..], env)
                }
                _ => todo!(),
            },
            _ => todo!(),
        },
        Sexp::Atom(Atom::IntLit(i)) => Value::Int(Val::new(*i)),
        Sexp::Atom(Atom::ArrayLit(exprs)) => {
            let first = eval_expression(&exprs[0], env);
            match first {
                Value::Int(val) => {
                    let mut v = Proxy::<Vec<_>>::new();
                    v.push(val);
                    for e in &exprs[1..] {
                        match eval_expression(e, env) {
                            Value::Int(e) => {
                                v.push(e);
                            }
                            _ => panic!("invalid value"),
                        }
                    }
                    Value::IntArray(v.into())
                }
                Value::Bool(val) => todo!(),
                Value::IntArray(slice) => todo!(),
                Value::Null => todo!(),
            }
        }
        Sexp::Atom(Atom::Ident(kw)) => match kw.as_str() {
            "true" => Value::Bool(Val::new(true)),
            "false" => Value::Bool(Val::new(false)),
            var => env.vars.get(var).unwrap().clone(),
            _ => unreachable!("unknown kw: {kw}"),
        },
    }
}

fn print_host<T: std::fmt::Display>(t: T) {
    println!("{t}");
}

fn print(t: Value) {
    match t {
        Value::Int(val) => {
            let host = print_host::<u64>.into_host_fn();
            host.call(val)
        }
        Value::Bool(_val) => {
            // let host = print_host::<bool>.into_host_fn();
            // host.call(val)
            todo!()
        }
        Value::IntArray(slice) => todo!(),
        Value::Null => todo!(),
    }
}

fn main() {
    // (select (+ foo 1) (+ foo bar) (from (foo u64) (bar u64)))
    let expr = "(foreach (lambda [x] (if (= x 2) (print x) (print (+ 2 x)))) [1 1 (+ 1 1)])";
    let e = dbg!(Parser::parse(expr));

    let mut ctx = Ctx::builder().build();
    let mut env = Env {
        vars: Default::default(),
    };
    let f = ctx.func::<(usize, usize), ()>(|_row| {
        eval_expression(&e[0], &mut env);
        ControlFlow::Break(())
    });

    let f = ctx.get_compiled_function(f);
    dbg!(f.call((10, 10)));
}

use parser::Parser;

use lego::prelude::*;

type Goal = Program;
type Program = Vec<Sexp>;
#[derive(Debug)]
enum Sexp {
    List(List),
    Atom(Atom),
}
type List = Vec<Sexp>;
type ListItems = Vec<Sexp>;
#[derive(Debug)]
enum Atom {
    IntLit(u64),
    Ident(String),
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
        <n:"[0-9]+"> => Atom::IntLit(n.parse().unwrap()),
        <id:"[a-zA-Z\\+=-][a-zA-Z0-9_]*"> => Atom::Ident(id.to_string()),
    };
}

enum Value {
    Int(Val<u64>),
    Bool(Val<bool>),
}

enum Type {
    Int,
    Bool,
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
            Value::Bool(_) => unreachable!(),
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

    fn sub(self, other: Value) -> Value {
        match (self, other) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs - rhs),
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

fn eval_if<T: AsType + ToPrimitive>(list: &[Sexp]) -> Value {
    let val = {
        {
            #[allow(unreachable_code)]
            lego::prelude::If::<_, _, _, (), _, _>::new(
                || eval_expression.fn_call((&list[1],)).into_bool(),
                |__ctx__| {
                    lego::prelude::ControlFlow::Break({
                        T::unwrap_val.fn_call((eval_expression(&list[2]),))
                    })
                },
                |__ctx__| {
                    lego::prelude::ControlFlow::Break({
                        T::unwrap_val.fn_call((eval_expression(&list[3]),))
                    })
                },
            )
            .eval()
        }
    }
    .into_break()
    .unwrap();

    T::into_value(val)
}

fn fold_list(op: impl Fn(Value, Value) -> Value, s: &[Sexp]) -> Value {
    let mut lhs = eval_expression(&s[0]);
    for exp in &s[1..] {
        let rhs = eval_expression(exp);
        lhs = op(lhs, rhs);
    }

    lhs
}

fn eval_expression(s: &Sexp) -> Value {
    match s {
        Sexp::List(list) => match &list[0] {
            Sexp::List(_vec) => todo!(),
            Sexp::Atom(Atom::Ident(s)) => match s.as_str() {
                "+" => fold_list(|a, b| a.add(b), &list[1..]),
                "-" => fold_list(|a, b| a.sub(b), &list[1..]),
                "if" => eval_if::<u64>(list),
                _ => todo!(),
            },
            _ => todo!(),
        },
        Sexp::Atom(Atom::IntLit(i)) => Value::Int(Val::new(*i)),
        Sexp::Atom(Atom::Ident(kw)) => {
            match kw.as_str() {
                "true" => Value::Bool(Val::new(true)),
                "false" => Value::Bool(Val::new(false)),
                _ => unreachable!("unknown kw: {kw}"),
            }
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
        },
        Value::Bool(_val) => {
            // let host = print_host::<bool>.into_host_fn();
            // host.call(val)
            todo!()
        },
    }
}

fn main() {
    // (select (+ foo 1) (+ foo bar) (from (foo u64) (bar u64)))
    let expr = "(if true (+ 1 1 1) 24)";
    let e = dbg!(Parser::parse(expr));

    let mut ctx = Ctx::builder().build();
    let f =
        ctx.func::<i32, u64>(|_row| ControlFlow::Break(u64::unwrap_val(eval_expression(&e[0]))));

    let f = ctx.get_compiled_function(f);
    dbg!(f.fn_call(12));
}

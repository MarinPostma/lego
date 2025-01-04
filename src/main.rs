use std::{cmp::Ordering, time::Instant};

use lego::prelude::*;

fn main() {
    let builder = Ctx::builder();
    let mut ctx = builder.build();

    let before = Instant::now();

    let mut count = 0;
    let mut main;
    loop {
        main = ctx.func::<i32, i32>(|val| {
            ControlFlow::Ret(
                lego!({
                    if val.neq(&100i32) {
                        if val.eq(&42i32) {
                            return Val::new(12i32)
                        }
                    }

                    val + 1i32
                })
            )
        });

        if count == 1000 {
            break;
        }

        count += 1;
    }
    dbg!(before.elapsed() / 1000);

    let main = ctx.get_compiled_function(main);

    let before = Instant::now();
    for _ in 0..1000 {
        main.call(42);
    }
    dbg!(before.elapsed());

    let mut vm = Vm {
        stack: Vec::with_capacity(32),
        code: vec![
            Inst::Param { idx: 0 },
            Inst::Push { val: 100 },
            Inst::Cmp { gotos: [7, 3, 7] },
            Inst::Param { idx: 0 },
            Inst::Push { val: 1 },
            Inst::Add,
            Inst::Ret,
            Inst::Param { idx: 0 },
            Inst::Push { val: 42 },
            Inst::Cmp { gotos: [3, 10, 3] },
            Inst::Push { val: 12 },
            Inst::Ret,
        ],
        ip: 0,
    };

    let before = Instant::now();
    for _ in 0..5000 {
        vm.run(&[100]);
        vm.ip = 0;
    }
    dbg!(before.elapsed());

    let mut vm = VmReg {
        regs: vec![0; 32],
        code: vec![
            InstR::Param { idx: 0, r: 0 },
            InstR::Const { v: 100, r: 1 },
            InstR::Cmp { rm: 0, rn: 1, jumps: [6, 3, 6] },
            InstR::Const { v: 1, r: 1 },
            InstR::Add { rd: 0, rm: 0, rn: 1 },
            InstR::Ret { r: 0 },
            InstR::Const { v: 42, r: 1 },
            InstR::Cmp { rm: 0, rn: 1, jumps: [9, 3, 9] },
            InstR::Const { v: 12, r: 0 },
            InstR::Ret { r: 0 },
        ],
        ip: 0,
    };

    let before = Instant::now();
    for _ in 0..5000 {
        vm.run(&[100]);
        vm.ip = 0;
    }
    dbg!(before.elapsed());
}

#[derive(Copy, Clone)]
enum Inst {
    Cmp { gotos: [usize; 3] },
    Push { val: u64 },
    Param { idx: usize },
    Add,
    Ret,
}

struct Vm {
    stack: Vec<u64>,
    code: Vec<Inst>,
    ip: usize,
}

impl Vm {
    fn run(&mut self, params: &[u64]) -> u64 {
        loop {
            match self.code[self.ip] {
                Inst::Cmp { gotos } => {
                    let rhs = self.stack.pop().unwrap();
                    let lhs = self.stack.pop().unwrap();
                    let idx = match lhs.cmp(&rhs) {
                        Ordering::Less => 0,
                        Ordering::Equal => 1,
                        Ordering::Greater => 2,
                    };
                    self.ip = gotos[idx];
                    continue
                },
                Inst::Push { val } => {
                    self.stack.push(val);
                },
                Inst::Add => {
                    let rhs = self.stack.pop().unwrap();
                    let lhs = self.stack.pop().unwrap();
                    self.stack.push(lhs + rhs);
                },
                Inst::Ret => return self.stack.pop().unwrap(),
                Inst::Param { idx } => {
                    let val = params[idx];
                    self.stack.push(val);
                },
            }

            self.ip += 1;
        }
    }
}

enum InstR {
    Param { idx: usize, r: usize },
    Const { v: u64, r: usize},
    Add { rd: usize, rm: usize, rn: usize },
    Cmp { rm: usize, rn: usize, jumps: [usize; 3] },
    Ret { r: usize },
}

struct VmReg {
    regs: Vec<u64>,
    code: Vec<InstR>,
    ip: usize,
}

impl VmReg {
    fn run(&mut self, params: &[u64]) -> u64 {
        loop {
            match self.code[self.ip] {
                InstR::Param { idx, r } => {
                    self.regs[r] = params[idx];
                },
                InstR::Const { v, r } => {
                    self.regs[r] = v;
                },
                InstR::Add { rd, rm, rn } => {
                    let lhs = self.regs[rm];
                    let rhs = self.regs[rn];
                    self.regs[rd] = lhs + rhs;

                },
                InstR::Cmp { rm, rn, jumps } => {
                    let lhs = self.regs[rm];
                    let rhs = self.regs[rn];
                    let idx = match lhs.cmp(&rhs) {
                        Ordering::Less => 0,
                        Ordering::Equal => 1,
                        Ordering::Greater => 2,
                    };
                    self.ip = jumps[idx];
                    continue
                },
                InstR::Ret { r } => return self.regs[r],
            }

            self.ip += 1;
        }
    }
}

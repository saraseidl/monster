use super::{
    Assignment, BVOperator, BitVector, Formula, FormulaVisitor, Solver, SolverError, SymbolId,
};
use std::{
    collections::HashMap,
    fs::File,
    io::{stdout, BufWriter, Write},
    path::Path,
    sync::{Arc, Mutex},
};

pub struct ExternalSolver {
    output: Arc<Mutex<dyn Write + Send>>,
}

impl ExternalSolver {
    pub fn new<P>(path: P) -> Result<Self, SolverError>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;

        let mut writer = BufWriter::new(file);

        write_init(&mut writer)?;

        let output = Arc::new(Mutex::new(writer));

        Ok(Self { output })
    }
}

fn write_init<W: Write>(writer: &mut W) -> Result<(), SolverError> {
    writeln!(writer, "(set-logic QF_BV)").map_err(SolverError::from)
}

impl Default for ExternalSolver {
    fn default() -> Self {
        let mut file = BufWriter::new(stdout());

        write_init(&mut file).expect("stdout should not fail");

        Self {
            output: Arc::new(Mutex::new(file)),
        }
    }
}

impl Solver for ExternalSolver {
    fn name() -> &'static str {
        "External"
    }

    fn solve_impl<F: Formula>(&self, formula: &F) -> Result<Option<Assignment>, SolverError> {
        {
            let mut output = self.output.lock().expect("no other thread should fail");

            writeln!(output, "(push 1)")?;

            // give lock back here
        }

        let mut printer = SmtPrinter {
            output: self.output.clone(),
        };
        let mut visited = HashMap::<SymbolId, Result<SymbolId, SolverError>>::new();

        formula.traverse(formula.root(), &mut visited, &mut printer)?;

        let mut output = self.output.lock().expect("no other thread should fail");

        writeln!(output, "(check-sat)\n(get-model)\n(pop 1)")?;

        Err(SolverError::SatUnknown)
    }
}

struct SmtPrinter {
    output: Arc<Mutex<dyn Write>>,
}

impl FormulaVisitor<Result<SymbolId, SolverError>> for SmtPrinter {
    fn input(&mut self, idx: SymbolId, name: &str) -> Result<SymbolId, SolverError> {
        let mut o = self.output.lock().expect("no other thread should fail");

        writeln!(o, "(declare-fun x{} () (_ BitVec 64)); {:?}", idx, name)?;

        Ok(idx)
    }

    fn constant(&mut self, idx: SymbolId, v: BitVector) -> Result<SymbolId, SolverError> {
        let mut o = self.output.lock().expect("no other thread should fail");

        writeln!(
            o,
            "(declare-fun x{} () (_ BitVec 64))\n(assert (= x{} (_ bv{} 64)))",
            idx, idx, v.0
        )?;

        Ok(idx)
    }

    fn unary(
        &mut self,
        idx: SymbolId,
        op: BVOperator,
        v: Result<SymbolId, SolverError>,
    ) -> Result<SymbolId, SolverError> {
        let mut o = self.output.lock().expect("no other thread should fail");

        writeln!(
            o,
            "(declare-fun x{} () (_ BitVec 64))\n(assert (= x{} ({} x{})))",
            idx,
            idx,
            to_smt(op),
            v?
        )?;

        Ok(idx)
    }

    fn binary(
        &mut self,
        idx: SymbolId,
        op: BVOperator,
        lhs: Result<SymbolId, SolverError>,
        rhs: Result<SymbolId, SolverError>,
    ) -> Result<SymbolId, SolverError> {
        let mut o = self.output.lock().expect("no other thread should fail");

        writeln!(
            o,
            "(declare-fun x{} () (_ BitVec 64))\n(assert (= x{} ({} x{} x{})))",
            idx,
            idx,
            to_smt(op),
            lhs?,
            rhs?
        )?;

        Ok(idx)
    }
}

fn to_smt(op: BVOperator) -> &'static str {
    match op {
        BVOperator::Add => "bvadd",
        BVOperator::Sub => "bvsub",
        BVOperator::Not => "not",
        BVOperator::Mul => "bvmul",
        BVOperator::Divu => "bvudiv",
        BVOperator::Remu => "bvurem",
        BVOperator::Equals => "=",
        BVOperator::BitwiseAnd => "bvand",
        BVOperator::Sltu => "bvult",
    }
}

use soul_utils::{CrateId, CrateStore, vec_map::VecMap};
use std::{fmt::Debug, time::Duration};

#[derive(Debug, Clone, Default)]
pub struct CrateBenchmark {
    pub source_read: Duration,
    pub tokenize: Duration,
    pub ast: Duration,
    pub hir: Duration,
    pub mir: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct Benchmarks {
    pub crates: VecMap<CrateId, CrateBenchmark>,
    pub ir: Duration,
    pub total: CrateBenchmark,
}

impl Benchmarks {
    pub fn source_read(&mut self, id: CrateId, time: Duration) {
        self.total.source_read += time;
        self.benchmark(id).source_read = time;
    }

    pub fn tokenize(&mut self, id: CrateId, time: Duration) {
        self.total.tokenize += time;
        self.benchmark(id).tokenize = time;
    }

    pub fn ast(&mut self, id: CrateId, time: Duration) {
        self.total.ast += time;
        self.benchmark(id).ast = time;
    }

    pub fn hir(&mut self, id: CrateId, time: Duration) {
        self.total.hir += time;
        self.benchmark(id).hir = time;
    }

    pub fn mir(&mut self, id: CrateId, time: Duration) {
        self.total.mir += time;
        self.benchmark(id).mir = time;
    }

    fn benchmark(&mut self, id: CrateId) -> &mut CrateBenchmark {
        self.crates.get_mut_or_default(id)
    }

    pub fn write_total(&self, sb: &mut impl std::fmt::Write) {
        let total = &self.total;

        let full_total = total.tokenize + total.ast + total.hir + total.mir + self.ir;

        let percent = |part: Duration, total: Duration| -> f64 {
            if total.is_zero() {
                0.0
            } else {
                (part.as_secs_f64() / total.as_secs_f64()) * 100.0
            }
        };

        writeln!(sb, "\n=== Full Pipeline Time ===").expect("no fmt error");
        writeln!(sb, "SourceRead: {}", fmt_duration(total.source_read)).expect("no fmt error");
        writeln!(sb, "Tokenizer:  {}", fmt_duration(total.tokenize)).expect("no fmt error");
        writeln!(sb, "AST:        {}", fmt_duration(total.ast)).expect("no fmt error");
        writeln!(sb, "HIR:        {}", fmt_duration(total.hir)).expect("no fmt error");
        writeln!(sb, "MIR:        {}", fmt_duration(total.mir)).expect("no fmt error");
        writeln!(sb, "LLVM IR:    {}", fmt_duration(self.ir)).expect("no fmt error");

        writeln!(sb, "\n=== Full Pipeline Percentages ===").expect("no fmt error");
        writeln!(sb, "SourceRead: {:.1}%", percent(total.source_read, full_total)).expect("no fmt error");
        writeln!(sb, "Tokenizer:  {:.1}%", percent(total.tokenize, full_total)).expect("no fmt error");
        writeln!(sb, "AST:        {:.1}%", percent(total.ast, full_total)).expect("no fmt error");
        writeln!(sb, "HIR:        {:.1}%", percent(total.hir, full_total)).expect("no fmt error");
        writeln!(sb, "MIR:        {:.1}%", percent(total.mir, full_total)).expect("no fmt error");
        writeln!(sb, "LLVM IR:    {:.1}%", percent(self.ir, full_total)).expect("no fmt error");

        writeln!(sb, "\nFull Total: {}", fmt_duration(full_total)).expect("no fmt error");
    }

    pub fn write_crates(&self, sb: &mut impl std::fmt::Write, store: &CrateStore) {
        for (id, b) in self.crates.entries() {
            let name = store
                .get(id)
                .map(|c| c.name.as_str())
                .unwrap_or("<unknown>");

            writeln!(sb, "\n=== Crate: {} ===", name).expect("no fmt error");
            writeln!(sb, "Tokenizer: {}", fmt_duration(b.tokenize)).expect("no fmt error");
            writeln!(sb, "AST:       {}", fmt_duration(b.ast)).expect("no fmt error");
            writeln!(sb, "HIR:       {}", fmt_duration(b.hir)).expect("no fmt error");
            writeln!(sb, "MIR:       {}", fmt_duration(b.mir)).expect("no fmt error");
        }
    }
}

fn fmt_duration(d: Duration) -> String {
    let ns = d.as_nanos();

    if ns < 1_000 {
        format!("{ns}ns")
    } else if ns < 1_000_000 {
        format!("{:.1}µs", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.4}ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.4}s", ns as f64 / 1_000_000_000.0)
    }
}

use soul_utils::{CrateId, CrateStore, vec_map::VecMap};
use std::{fmt::Debug, time::Duration};

#[derive(Debug, Clone, Default)]
pub struct CrateBenchmark {
    pub read: Duration,
    pub tokens: Duration,
    pub ast: Duration,
    pub hir: Duration,
    pub mir: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct Benchmarks {
    pub ir: Duration,
    pub total: CrateBenchmark,
    pub crates: VecMap<CrateId, CrateBenchmark>,
}

impl Benchmarks {
    pub const fn const_default() -> Self {
        Self {
            ir: Duration::new(0, 0),
            total: CrateBenchmark::const_default(),
            crates: VecMap::const_default(),
        }
    }

    pub fn source_read(&mut self, id: CrateId, time: Duration) {
        self.total.read += time;
        self.benchmark(id).read = time;
    }

    pub fn tokenize(&mut self, id: CrateId, time: Duration) {
        self.total.tokens += time;
        self.benchmark(id).tokens = time;
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

    pub fn write_total(&self, sb: &mut impl std::fmt::Write) -> std::fmt::Result {
        let total = &self.total;

        let full_total = total.tokens + total.ast + total.hir + total.mir + self.ir;

        writeln!(sb, "\n=== Full Pipeline Time ===")?;
        writeln!(sb, "SourceRead: {}", fmt_duration(total.read))?;
        writeln!(sb, "Tokenizer:  {}", fmt_duration(total.tokens))?;
        writeln!(sb, "AST:        {}", fmt_duration(total.ast))?;
        writeln!(sb, "HIR:        {}", fmt_duration(total.hir))?;
        writeln!(sb, "MIR:        {}", fmt_duration(total.mir))?;
        writeln!(sb, "LLVM IR:    {}", fmt_duration(self.ir))?;

        writeln!(sb, "\n=== Full Pipeline Percentages ===")?;
        writeln!(sb, "SourceRead: {:.1}%", percent(total.read, full_total))?;
        writeln!(sb, "Tokenizer:  {:.1}%", percent(total.tokens, full_total))?;
        writeln!(sb, "AST:        {:.1}%", percent(total.ast, full_total))?;
        writeln!(sb, "HIR:        {:.1}%", percent(total.hir, full_total))?;
        writeln!(sb, "MIR:        {:.1}%", percent(total.mir, full_total))?;
        writeln!(sb, "LLVM IR:    {:.1}%", percent(self.ir, full_total))?;

        writeln!(sb, "\nFull Total: {}", fmt_duration(full_total))
    }

    pub fn write_crates(
        &self,
        sb: &mut impl std::fmt::Write,
        store: &CrateStore,
    ) -> std::fmt::Result {
        for (id, b) in self.crates.entries() {
            let name = store
                .get(id)
                .map(|c| c.name.as_str())
                .unwrap_or("<unknown>");

            writeln!(sb, "\n=== Crate: {} ===", name)?;
            writeln!(sb, "Tokenizer: {}", fmt_duration(b.tokens))?;
            writeln!(sb, "AST:       {}", fmt_duration(b.ast))?;
            writeln!(sb, "HIR:       {}", fmt_duration(b.hir))?;
            writeln!(sb, "MIR:       {}", fmt_duration(b.mir))?;
        }

        Ok(())
    }
}

impl CrateBenchmark {
    pub const fn const_default() -> Self {
        const DEFAULT: Duration = Duration::new(0, 0);
        Self {
            read: DEFAULT,
            tokens: DEFAULT,
            ast: DEFAULT,
            hir: DEFAULT,
            mir: DEFAULT,
        }
    }
}

fn percent(part: Duration, total: Duration) -> f64 {
    if total.is_zero() {
        return 0.0;
    }

    (part.as_secs_f64() / total.as_secs_f64()) * 100.0
}

fn fmt_duration(d: Duration) -> String {
    let nano = d.as_nanos();

    match nano {
        0..1_000 => format!("{nano}ns"),
        1_000..1_000_000 => format!("{:.1}µs", nano as f64 / 1_000.0),
        1_000_000..1_000_000_000 => format!("{:.4}ms", nano as f64 / 1_000_000.0),
        _ => format!("{:.4}s", nano as f64 / 1_000_000_000.0),
    }
}

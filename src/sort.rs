#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Pid,
    Name,
    Cpu,
    Memory,
    Threads,
}

impl SortBy {
    pub fn cycle(&self) -> Self {
        match self {
            SortBy::Pid => SortBy::Name,
            SortBy::Name => SortBy::Cpu,
            SortBy::Cpu => SortBy::Memory,
            SortBy::Memory => SortBy::Threads,
            SortBy::Threads => SortBy::Pid,
        }
    }
}

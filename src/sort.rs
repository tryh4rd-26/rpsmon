#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Pid,
    Name,
    Cpu,
    Memory,
    Threads,
}

impl SortBy {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "pid" => SortBy::Pid,
            "name" => SortBy::Name,
            "memory" | "mem" => SortBy::Memory,
            "threads" | "thread" => SortBy::Threads,
            _ => SortBy::Cpu,
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            SortBy::Pid => SortBy::Name,
            SortBy::Name => SortBy::Cpu,
            SortBy::Cpu => SortBy::Memory,
            SortBy::Memory => SortBy::Threads,
            SortBy::Threads => SortBy::Pid,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortBy::Pid => "pid",
            SortBy::Name => "name",
            SortBy::Cpu => "cpu",
            SortBy::Memory => "memory",
            SortBy::Threads => "threads",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SortBy;

    #[test]
    fn parse_sort_mode_from_config() {
        assert_eq!(SortBy::from_str("cpu"), SortBy::Cpu);
        assert_eq!(SortBy::from_str("PID"), SortBy::Pid);
        assert_eq!(SortBy::from_str("mem"), SortBy::Memory);
        assert_eq!(SortBy::from_str("unknown"), SortBy::Cpu);
    }
}

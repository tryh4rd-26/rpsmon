/// Simple sparkline rendering using Unicode block characters
#[allow(dead_code)]
pub struct Sparkline;

#[allow(dead_code)]
impl Sparkline {
    const BLOCKS: &'static [char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    pub fn render(values: &[f32], width: usize) -> String {
        if values.is_empty() {
            return String::new();
        }

        let max = values.iter().copied().fold(0.0, f32::max);
        if max == 0.0 {
            return "▁".repeat(width);
        }

        // Resample values to fit width
        let step = if values.len() > width {
            values.len() as f32 / width as f32
        } else {
            1.0
        };

        (0..width)
            .map(|i| {
                let idx = (i as f32 * step) as usize;
                let val = values.get(idx).copied().unwrap_or(0.0);
                let block_idx = ((val / max * 7.0).round() as usize).min(7);
                Self::BLOCKS[block_idx]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_render() {
        let values = vec![0.0, 25.0, 50.0, 75.0, 100.0];
        let result = Sparkline::render(&values, 5);
        assert_eq!(result.chars().count(), 5);
    }
}

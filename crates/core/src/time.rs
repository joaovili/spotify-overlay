/// `mm:ss`, como nas versoes antigas (faixas do Spotify nao passam de 59:59).
pub fn mmss(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(mmss(0.0), "0:00");
        assert_eq!(mmss(59.9), "0:59");
        assert_eq!(mmss(60.0), "1:00");
        assert_eq!(mmss(3599.0), "59:59");
        assert_eq!(mmss(-5.0), "0:00");
    }
}

#![deny(missing_docs)]
#![doc = "SerinDB core library."]

/// Returns `true` if the library is properly linked and functioning.
///
/// # Examples
///
/// ```
/// assert_eq!(serindb::health_check(), true);
/// ```
pub fn health_check() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_returns_true() {
        assert!(health_check());
    }
} 
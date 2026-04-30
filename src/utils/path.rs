/// Normalizes a path string by resolving `.` and `..` components.
///
/// This performs lexical normalization only. It does not access the filesystem,
/// canonicalize symlinks, or require the path to exist. Current-directory
/// markers are skipped, parent-directory markers pop the previous normalized
/// component, and all other components are preserved.
pub fn normalize(source: &String) -> std::path::PathBuf {
    let input = std::path::PathBuf::from(source);

    let mut new_path = std::path::PathBuf::new();

    for component in input.components() {
        match component {
            // Skip the current-dir marker "."
            std::path::Component::CurDir => {}

            // For "..", pop the last component if possible
            std::path::Component::ParentDir => {
                new_path.pop();
            }

            // For normal components, push them
            other => new_path.push(other.as_os_str()),
        }
    }

    new_path
}

#[cfg(test)]
mod test {
    use crate::utils::path::normalize;

    #[test]
    fn test_current_directory_marker() {
        let subject = ".".to_string();

        let results = normalize(&subject);

        assert_eq!(
            results,
            std::path::PathBuf::new(),
            "it should skip current directory marker"
        );
    }

    #[test]
    fn test_parent_directory_marker() {
        let subject = "../some-dir".to_string();

        let results = normalize(&subject);

        assert_eq!(
            results,
            std::path::PathBuf::from("some-dir"),
            "it should skip leading parent directory markers when there is no component to pop"
        );

        let subject = "some-dir/../other-dir".to_string();

        let results = normalize(&subject);

        assert_eq!(
            results,
            std::path::PathBuf::from("other-dir"),
            "it should pop the previous component for parent directory markers"
        );
    }

    #[test]
    fn test_normal_path() {
        let subject = "some-dir/other-dir/file.txt".to_string();

        let results = normalize(&subject);

        assert_eq!(
            results,
            std::path::PathBuf::from("some-dir/other-dir/file.txt"),
            "it should preserve normal path components"
        );
    }
}

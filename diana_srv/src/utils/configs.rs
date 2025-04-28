pub mod server {
    use crate::utils::readers::files;
    use std::path::Path;
    pub fn config_toml(path: &String) -> &Path {
        assert!(files::check_if_file_exists(path));
        Path::new(path)
    }
}

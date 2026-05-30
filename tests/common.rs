use std::path::Path;

pub fn create_runtime() -> oolong::runtime::OolongRuntime {
    oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap()
}

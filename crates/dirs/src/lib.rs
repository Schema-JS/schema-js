use std::path::PathBuf;

pub const BASE_SCHEME_JS_FOLDER: &str = ".scheme-js";

pub fn get_base_path() -> PathBuf {
    dirs::data_dir().unwrap().join(BASE_SCHEME_JS_FOLDER)
}
pub fn create_scheme_js_folder() {
    let paths = [get_base_path(), get_base_path().join("dbs")].into_iter();

    for path in paths {
        println!("{}", path.to_str().unwrap());
        if !path.exists() {
            std::fs::create_dir(path).unwrap();
        }
    }
}

pub fn create_scheme_js_db(db_name: &str) -> PathBuf {
    let path = get_base_path().join("dbs").join(db_name);

    if !path.exists() {
        std::fs::create_dir(path.clone()).unwrap();
    }

    path
}

pub fn create_schema_js_table(db_name: &str, table_name: &str) -> PathBuf {
    let path = get_base_path().join("dbs").join(db_name).join(table_name);

    if !path.exists() {
        std::fs::create_dir(path.clone()).unwrap();
    }

    path
}

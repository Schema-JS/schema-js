use colored::Colorize;
use std::io::Write;
use std::path::PathBuf;

static USERS_TABLE_FILE_CONTENT: &str = r#"
export default function main() {
    const { Table, Column } = SchemaJS;
    return new Table("users")
        .addColumn(new Column("id").string())
        .addColumn(new Column("username").string())
        .addColumn(new Column("password").string())
        .addColumn(new Column("enabled").boolean().withDefaultValue(true));
}
"#;

pub(crate) struct InitOpts {
    pub(crate) dir: Option<String>,
}
pub(crate) fn init_cmd(opts: InitOpts) {
    let InitOpts { dir } = opts;

    let dir_path = dir
        .map(|p| PathBuf::from(p))
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    if !dir_path.exists() {
        if let Err(err) = std::fs::create_dir_all(&dir_path) {
            eprintln!(
                "[{}] Folder could not be found or created: {:?}. Error: {:?}",
                "Error".red(),
                dir_path,
                err
            );
            return;
        }
    }

    println!("[{}] Working on directory {:?}", "Info".yellow(), dir_path);
    println!();

    let schema_js_toml = dir_path.join("SchemaJS.toml");
    let public_schema_tables = dir_path.join("public/tables");

    if let Err(err) = std::fs::File::create(&schema_js_toml) {
        eprintln!(
            "[{}] SchemaJS.toml could not be created. Error: {:?}",
            "Error".red(),
            err
        );
        return;
    }

    if let Err(err) = std::fs::create_dir_all(&public_schema_tables) {
        eprintln!(
            "[{}] 'public' schema tables could not be created. Error: {:?}",
            "Error".red(),
            err
        );
        return;
    }

    let users_table = public_schema_tables.join("users.ts");

    if let Err(err) = std::fs::File::create(&users_table)
        .and_then(|mut file| file.write_all(USERS_TABLE_FILE_CONTENT.as_bytes()))
    {
        eprintln!(
            "[{}] Default table 'users.ts' could not be created or initialized: {:?}",
            "Error".red(),
            err
        );
        return;
    }

    println!();
    println!("[{}] SchemaJS initialized successfully", "Success".green());
    println!();

    println!("To start the server, run:");
    println!("  cd {:?} && schemajs start", dir_path);
    println!();
    println!("Stuck? Join our Discord https://discord.gg/nRzTHygKn5");
    println!();
}

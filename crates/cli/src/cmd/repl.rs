use base::runner::SjsRunner;
use base::runtime::SchemeJsRuntime;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor};
use schemajs_core::GlobalContext;
use schemajs_repl::errors::{ReplError, ReplErrorResponse};
use schemajs_repl::query_state::ReplQueryState;
use schemajs_repl::{get_current_db_context, get_query_state, run_repl_script};
use serde_json::Value;
use std::sync::Arc;

enum InputResult {
    Line(String),
    CtrlC,
    CtrlD,
    Error(String),
}

fn handle_repl_error(val: &Value) -> bool {
    if let Some(_) = val.get("REPL_ERR") {
        let err = ReplErrorResponse::from(val);
        match err.error {
            ReplError::AlreadyInContext => {
                println!(
                    "[{}] Already in database context. Exit by calling `exit()` or by doing `use(dbName, tableName)`",
                    "Info".yellow()
                );
            }
            ReplError::UnexpectedUseArgsLength => {
                println!(
                    "[{}] Method `use` is expecting two arguments.",
                    "Error".red()
                );
            }
            ReplError::AlreadyInGlobal => {
                println!(
                    "[{}] Already in global context. Press CTRL+C or type `close()` to exit.",
                    "Info".yellow()
                );
            }
        }
        return true;
    }

    false
}

pub(crate) async fn repl(runner: Arc<SjsRunner>) {
    runner.sjs_context.mark_repl();

    let mut rt = SchemeJsRuntime::new(runner.sjs_context.clone())
        .await
        .unwrap();

    println!("> {}", "REPL is running".yellow());
    println!();

    let mut context = GlobalContext::default();
    let mut rl = DefaultEditor::new().unwrap();

    loop {
        let current_query_state = get_query_state(&context);

        if context.repl_exit {
            break;
        }

        let cmd_prefix = {
            match &current_query_state {
                ReplQueryState::Global => format!("({}) > ", "global".green()),
                ReplQueryState::Database(db) => format!("({}) > ", db.green()),
                ReplQueryState::Table(db, tbl) => format!("({}.{}) > ", db.green(), tbl.blue()),
            }
        };

        let input = get_user_input(cmd_prefix.as_str(), &mut rl);

        match input {
            InputResult::Line(input) => {
                let result = run_repl_script(&mut rt.js_runtime, input).await;
                context = get_current_db_context(&mut rt.js_runtime);

                if let Ok(res) = result {
                    if let Some(res) = res {
                        let err = handle_repl_error(&res);
                        if !err {
                            if !res.is_null() {
                                println!("{}", res);
                            }
                        }
                    }
                } else {
                    let err = result.err().unwrap();
                    println!("[{}] {}", "Error".red(), err);
                }
                println!();
            }
            InputResult::CtrlC => {
                break;
            }
            InputResult::CtrlD => {
                break;
            }
            InputResult::Error(e) => {
                println!("[Error] {}", e.red());
                break;
            }
        }
    }
}

fn get_user_input(prompt: &str, history: &mut Editor<(), DefaultHistory>) -> InputResult {
    match history.readline(prompt) {
        Ok(line) => {
            let _ = history.add_history_entry(line.as_str());
            InputResult::Line(line.trim().to_string()) // Remove leading/trailing spaces
        }
        Err(ReadlineError::Interrupted) => InputResult::CtrlC,
        Err(ReadlineError::Eof) => InputResult::CtrlD,
        Err(err) => InputResult::Error(err.to_string()),
    }
}

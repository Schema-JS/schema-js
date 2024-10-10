use base::runner::SjsRunner;
use base::runtime::SchemeJsRuntime;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor};
use std::sync::Arc;

enum ReplQueryState {
    Global,
    Database(String),      // Holds the current database name
    Table(String, String), // Holds both the current database and table names
}

enum InputResult {
    Line(String),
    CtrlC,
    CtrlD,
    Error(String),
}

fn handle_use_command(input: &str, current_query_state: &mut ReplQueryState) {
    if input.starts_with("use ") {
        let name = input.trim_start_matches("use ").to_string();
        match &current_query_state {
            ReplQueryState::Global => {
                *current_query_state = ReplQueryState::Database(name);
            }
            ReplQueryState::Database(db) => {
                *current_query_state = ReplQueryState::Table(db.clone(), name);
            }
            ReplQueryState::Table(db_name, _) => {
                *current_query_state = ReplQueryState::Table(db_name.clone(), name);
            }
        }
    }
}

fn handle_exit_command(current_query_state: &mut ReplQueryState) {
    match current_query_state {
        ReplQueryState::Table(db_name, _) => {
            *current_query_state = ReplQueryState::Database(db_name.clone());
        }
        ReplQueryState::Database(_) => {
            *current_query_state = ReplQueryState::Global;
        }
        ReplQueryState::Global => {
            println!(
                "[{}] Already in global context. Press CTRL+C to exit.",
                "Info".yellow()
            );
        }
    }
}

fn set_db_context(rt: &mut SchemeJsRuntime, db: Option<String>, tbl: Option<String>) {
    let _ = rt.raw_set_db_context(&db, &tbl);
}

pub(crate) async fn repl(runner: Arc<SjsRunner>) {
    runner.sjs_context.mark_repl();

    let mut rt = SchemeJsRuntime::new(runner.sjs_context.clone())
        .await
        .unwrap();

    println!("> {}", "REPL is running".yellow());
    println!();

    let mut current_query_state = ReplQueryState::Global;
    let mut rl = DefaultEditor::new().unwrap();

    loop {
        let cmd_prefix = {
            match &current_query_state {
                ReplQueryState::Global => {
                    format!("({}) > ", "global".green())
                }
                ReplQueryState::Database(db) => {
                    set_db_context(&mut rt, Some(db.clone()), None);

                    format!("({}) > ", db.green())
                }
                ReplQueryState::Table(db, tbl) => {
                    set_db_context(&mut rt, Some(db.clone()), Some(tbl.clone()));

                    format!("({}.{}) > ", db.green(), tbl.blue())
                }
            }
        };

        let input = get_user_input(cmd_prefix.as_str(), &mut rl);

        match input {
            InputResult::Line(input) => {
                if input.starts_with("use ") {
                    handle_use_command(&input, &mut current_query_state);
                } else if input == "exit" {
                    handle_exit_command(&mut current_query_state);
                } else {
                    let result = rt.run_repl_script(input).await;

                    if let Ok(res) = result {
                        println!("{}", res.unwrap_or_else(|| serde_json::Value::Null))
                    } else {
                        let err = result.err().unwrap();
                        println!("[{}] {}", "Error".red(), err);
                    }
                }
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

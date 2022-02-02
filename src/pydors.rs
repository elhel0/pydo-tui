use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
const INIT_TODO: &str = "{\n    \"remember-header\": \"\\n REMEMBER\\n --------\",\n    \"remember-items\": [\n    \n    ],\n    \"header\": \"\\n TODO\\n ----\",\n    \"tasks\": [\n    ],\n    \"completed-tasks\": 0,\n    \"unfinished-tasks\": 0\n}";

pub fn parse(args: Vec<String>) {
    let fp = get_path();
    if Path::new(&fp).exists() {
        let contents = fs::read_to_string(&fp).unwrap();
        let todo = json::parse(&contents).unwrap();
        if args.len() == 1 {
            return;
        } else {
            match args[1].as_str() {
                "-a" => add(&fp, &args, todo),
                "add" => add(&fp, &args, todo),
                "-res" => reset_todo(&fp),
                "reset" => reset_todo(&fp),
                "-r" => remove_line(&fp, &args, todo),
                "remove" => remove_line(&fp, &args, todo),
                "-c" => complete_todo(&fp, &args, todo),
                "complete" => complete_todo(&fp, &args, todo),
                "-rem" => remember_todo(&fp, &args, todo),
                "remember" => remember_todo(&fp, &args, todo),
                "-remr" => remove_remember(&fp, &args, todo),
                "remove-remember" => remove_remember(&fp, &args, todo),
                _ => {}
            }
        }
    } else {
        init_todo(&fp);
    }
}
fn get_path() -> PathBuf {
    let dir = env::current_dir().unwrap();
    return dir.join("pydo.td");
}
fn write_todo(fp: &PathBuf, content: &str) {
    let mut file = File::create(&fp).unwrap();
    writeln!(&mut file, "{}", content).unwrap();
}
fn reset_todo(fp: &PathBuf) {
    write_todo(&fp, INIT_TODO);
}
fn init_todo(fp: &PathBuf) {
    write_todo(fp, INIT_TODO);
}
fn remember_todo(fp: &PathBuf, args: &Vec<String>, mut todo: json::JsonValue) {
    let mut remember = json::JsonValue::new_object();
    if args.len() >= 3 {
        let task_title = args[2].as_str();
        remember["item"] = task_title.into();
    } else {
        remember["item"] = "".into();
    }
    todo["remember-items"].push(remember).unwrap_or_default();
    write_todo(fp, todo.dump().as_str());
}
fn remove_completed(fp: &PathBuf, mut todo: json::JsonValue) {
    if todo["tasks"].len() == 0 {
        return;
    }
    let mut i = todo["tasks"].len() - 1;
    loop {
        if todo["tasks"][i]["completed"] == true {
            todo["tasks"].array_remove(i);
        }
        if i == 0 {
            break;
        } else {
            i -= 1;
        }
    }
    write_todo(fp, todo.dump().as_str());
}
fn remove_remember(fp: &PathBuf, args: &Vec<String>, mut todo: json::JsonValue) {
    if args.len() >= 3 {
        let index: usize = args[2].parse().unwrap_or(10000);
        if index < todo["remember-items"].len() {
            todo["remember-items"].array_remove(index);
            write_todo(fp, todo.dump().as_str());
        }
    }
}
fn add(fp: &PathBuf, args: &Vec<String>, mut todo: json::JsonValue) {
    let mut task = json::JsonValue::new_object();
    let unfinished = &todo["unfinished-tasks"].as_i32().unwrap_or(0) + 1;
    if args.len() >= 3 {
        let task_title = args[2].as_str();
        task["task"] = task_title.into();
    } else {
        task["task"] = "".into();
    }
    task["completed"] = false.into();
    todo["tasks"].push(task).unwrap_or_default();
    todo["unfinished-tasks"] = unfinished.into();
    write_todo(fp, todo.dump().as_str());
}
fn complete_todo(fp: &PathBuf, args: &Vec<String>, mut todo: json::JsonValue) {
    if args.len() >= 3 {
        let index: usize = args[2].parse().unwrap_or(10000);
        if index < todo["tasks"].len() {
            if todo["tasks"][index]["completed"] == false {
                todo["tasks"][index]["completed"] = true.into();
                let completed = todo["completed-tasks"].as_i32().unwrap_or(0) + 1;
                todo["completed-tasks"] = completed.into();
            } else {
                todo["tasks"][index]["completed"] = false.into();
                let completed = todo["completed-tasks"].as_i32().unwrap_or(0) - 1;
                todo["completed-tasks"] = completed.into();
            }
            write_todo(fp, todo.dump().as_str());
        }
    }
}
fn remove_line(fp: &PathBuf, args: &Vec<String>, mut todo: json::JsonValue) {
    if args.len() >= 3 {
        let index: usize = args[2].parse().unwrap_or(10000);
        if index < todo["tasks"].len() {
            if todo["tasks"][index]["completed"] == false {
                let unfinished = todo["unfinished-tasks"].as_i32().unwrap_or(0) - 1;
                todo["unfinished-tasks"] = unfinished.into();
            }
            todo["tasks"].array_remove(index);
            write_todo(fp, todo.dump().as_str());
        } else if args[2] == "all" {
            remove_completed(fp, todo);
        }
    }
}

#![no_std]
#![no_main]
#![doc = include_str!("../README.md")]

#[macro_use]
extern crate axlog;
extern crate alloc;
extern crate axruntime;

mod entry;
mod mm;
mod syscall;

use alloc::{string::String, vec::Vec};

fn parse_cmd(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;

    for c in cmd.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            _ => current_arg.push(c),
        }
    }

    if !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}

#[unsafe(no_mangle)]
fn main() {
    let testcases = option_env!("AX_TESTCASES_LIST")
        .unwrap_or_else(|| "Please specify the testcases list by making user_apps")
        .split(',')
        .filter(|&x| !x.is_empty());

    for testcase in testcases {
        if testcase.is_empty() || testcase.starts_with('#') {
            // Skip empty lines and comments
            continue;
        }
        let args = parse_cmd(testcase);
        if args.is_empty() {
            continue;
        }
        info!("Running user task: {}", testcase);
        info!("Arguments: {:?}", args);
        let exit_code = entry::run_user_app(&args, &[]);
        info!("User task {} exited with code: {:?}", testcase, exit_code);
    }
}

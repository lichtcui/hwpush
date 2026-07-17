use clap::Parser;

mod cli;
mod config;
mod core;
mod storage;
mod template;

/// hiboard — 将任务结果推送到华为负一屏
#[derive(Parser, Debug)]
#[command(name = "hiboard", version, about = "将任务结果推送到华为负一屏")]
struct Cli {
    #[command(subcommand)]
    command: cli::Command,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = cli::dispatch(cli.command) {
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}

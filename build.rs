use clap::CommandFactory;
use clap_mangen::Man;
use std::env;
use std::path::Path;

#[path = "src/cli.rs"]
mod cli;

fn main() -> std::io::Result<()> {
    let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or("target".to_string());
    let output_dir = Path::new(&target_dir).join(env::var("PROFILE").unwrap());
    let cmd = cli::Cli::command();
    let man = Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    std::fs::write(output_dir.join("git-req.1"), buffer)?;
    Ok(())
}

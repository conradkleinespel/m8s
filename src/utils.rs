use log::debug;
use std::io;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub fn run_command_with_piped_stdio(program: &str, args: &[&str], dry_run: bool) -> io::Result<()> {
    debug!("Running command {} {:?}", program, args);

    if dry_run {
        return Ok(());
    }

    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);
    let mut output_stdout = String::new();
    for line in reader.lines() {
        let line = line?;
        output_stdout.push_str(&line);
        output_stdout.push('\n');
        println!("{}", line);
    }

    let stderr = child.stderr.take().expect("Failed to capture stderr");
    let reader = BufReader::new(stderr);
    let mut output_stderr = String::new();
    for line in reader.lines() {
        let line = line?;
        output_stderr.push_str(&line);
        output_stderr.push('\n');
        println!("{}", line);
    }

    if !child.wait()?.success() {
        return Err(io::Error::new(io::ErrorKind::Other, output_stderr));
    }
    Ok(())
}

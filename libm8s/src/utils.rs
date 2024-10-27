use log::debug;
use std::io;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub(crate) fn run_command_with_piped_stdio(
    program: &str,
    args: &[&str],
    kubeconfig: Option<String>,
    dry_run: bool,
) -> io::Result<()> {
    debug!("Running command {} {:?}", program, args);

    if dry_run {
        return Ok(());
    }

    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(c) = kubeconfig {
        command.env("KUBECONFIG", c.to_string());
    }

    let mut child = command.spawn().expect("Failed to execute command");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let output_stdout = Arc::new(Mutex::new(String::new()));
    let output_stderr = Arc::new(Mutex::new(String::new()));

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let stdout_handle = {
        let output_stdout = Arc::clone(&output_stdout);
        std::thread::spawn(move || {
            for line in stdout_reader.lines() {
                let line = line.expect("Failed to read line from stdout");
                println!("{}", line);
                output_stdout.lock().unwrap().push_str(&line);
                output_stdout.lock().unwrap().push('\n');
            }
        })
    };

    let stderr_handle = {
        let output_stderr = Arc::clone(&output_stderr);
        std::thread::spawn(move || {
            for line in stderr_reader.lines() {
                let line = line.expect("Failed to read line from stderr");
                eprintln!("{}", line);
                output_stderr.lock().unwrap().push_str(&line);
                output_stderr.lock().unwrap().push('\n');
            }
        })
    };

    stdout_handle.join().expect("Failed to join stdout thread");
    stderr_handle.join().expect("Failed to join stderr thread");

    if !child.wait()?.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            output_stderr.lock().unwrap().to_string(),
        ));
    }
    Ok(())
}

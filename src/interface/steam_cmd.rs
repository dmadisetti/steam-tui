use crate::util::error::STError;

use std::process;

use std::io::{BufRead, BufReader, Write};

pub struct SteamCmd {
    iter: std::io::Split<BufReader<process::ChildStdout>>,
    stdin: process::ChildStdin,
}

impl SteamCmd {
    fn with_args_and_seperator(args: Vec<&str>, sep: u8) -> Result<SteamCmd, STError> {
        let attempt = process::Command::new("steamcmd")
            .args(args.as_slice())
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .spawn();

        if let Err(err) = attempt {
            return Err(STError::Process(err));
        }
        let child = attempt?;

        let f = BufReader::new(
            child
                .stdout
                .ok_or_else(|| STError::Problem("Failed to attach to stdout.".to_string()))?,
        );
        let iter = f.split(sep);
        let stdin = child
            .stdin
            .ok_or_else(|| STError::Problem("Failed to attach to stdin..".to_string()))?;

        Ok(SteamCmd { iter, stdin })
    }

    fn with_args(args: Vec<&str>) -> Result<SteamCmd, STError> {
        let mut cmd = SteamCmd::with_args_and_seperator(args, 0x1b)?;

        // Send start up data I guess yeah?
        cmd.next();
        cmd.next();
        cmd.next();
        cmd.next();

        Ok(cmd)
    }

    pub fn new() -> Result<SteamCmd, STError> {
        SteamCmd::with_args(vec![
            "+@ShutdownOnFailedCommand 0",
            "+@NoPromptForPassword 1",
        ])
    }

    pub fn script(script: &str) -> Result<SteamCmd, STError> {
        SteamCmd::with_args_and_seperator(
            vec![
                "+@ShutdownOnFailedCommand 1",
                "+@NoPromptForPassword 1",
                "+@sStartupScript",
                &format!("runscript {}", script),
            ],
            0x0a,
        )
    }
    pub fn write(&mut self, line: &str) -> Result<(), STError> {
        // Strip line endings
        let line: String = line.chars().filter(|&c| !"\n\r".contains(c)).collect();
        let line = format!("{}\n", line);
        self.stdin.write_all(line.as_bytes())?;
        Ok(())
    }
    pub fn maybe_next(&mut self) -> Result<Vec<u8>, STError> {
        match self.next() {
            Some(Ok(result)) => Ok(result),
            _ => Err(STError::Problem("Unable to read from stdin".into())),
        }
    }
}

impl Iterator for SteamCmd {
    type Item = Result<Vec<u8>, std::io::Error>;
    fn next(&mut self) -> Option<Result<Vec<u8>, std::io::Error>> {
        self.iter.next()
    }
}

impl Drop for SteamCmd {
    fn drop(&mut self) {
        // Failure is fine, because stopping anyway.
        let _ = self.write(&String::from("quit\n"));
    }
}

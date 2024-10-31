use crate::utils::CommandRunner;
use libm8s::file_format::create_json_schema;
use std::io;

pub struct CommandJsonSchema {}

impl CommandRunner for CommandJsonSchema {
    fn run(&self) -> io::Result<()> {
        println!("{}", create_json_schema()?);
        Ok(())
    }
}

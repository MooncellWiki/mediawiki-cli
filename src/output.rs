use anyhow::Result;
use serde::Serialize;

/// Output format for command results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl OutputFormat {
    pub fn is_json(self) -> bool {
        matches!(self, OutputFormat::Json)
    }
}

pub fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Print a list of lines: as a JSON array in JSON mode, one per line otherwise.
pub fn print_lines(lines: &[String], format: OutputFormat) -> Result<()> {
    if format.is_json() {
        print_json(lines)
    } else {
        for line in lines {
            println!("{line}");
        }
        Ok(())
    }
}

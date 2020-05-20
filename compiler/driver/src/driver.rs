use std::env::ArgsOs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;

use codespan_reporting::term::termcolor::WriteColor;
use codespan_reporting::term::Config;

use liblumen_session::{CodegenOptions, DebuggingOptions};

use crate::argparser;
use crate::commands;

pub fn run_compiler_with_emitter(
    cwd: PathBuf,
    args: ArgsOs,
    output_writer: Arc<Mutex<dyn WriteColor>>,
    error_writer: Arc<Mutex<dyn WriteColor>>,
    emit_config: Arc<Config>,
) -> anyhow::Result<()> {
    use liblumen_session::OptionGroup;

    // Parse arguments
    let matches = argparser::parse(args)?;

    // Parse option groups first, as they can produce usage
    let c_opts = CodegenOptions::parse_option_group(&matches)?.unwrap_or_else(Default::default);
    let z_opts = DebuggingOptions::parse_option_group(&matches)?.unwrap_or_else(Default::default);

    // Dispatch to the command implementation
    match matches.subcommand() {
        ("print", subcommand_matches) => commands::print::handle_command(
            c_opts,
            z_opts,
            subcommand_matches.unwrap(),
            cwd,
            output_writer,
            error_writer,
            emit_config,
        ),
        ("compile", subcommand_matches) => commands::compile::handle_command(
            c_opts,
            z_opts,
            subcommand_matches.unwrap(),
            cwd,
            output_writer,
            error_writer,
            emit_config,
        ),
        (subcommand, _) => Err(anyhow!(format!("Unrecognized subcommand '{}'", subcommand))),
    }
}

pub(crate) mod compile;
pub(crate) mod print;

use std::sync::{Arc, Mutex};

use codespan_reporting::term::termcolor::WriteColor;
use codespan_reporting::term::Config;

use libeir_diagnostics::CodeMap;

use liblumen_session::{DiagnosticsConfig, DiagnosticsHandler, Options};

pub(super) fn default_diagnostics_handler(
    options: &Options,
    output_writer: Arc<Mutex<dyn WriteColor>>,
    error_writer: Arc<Mutex<dyn WriteColor>>,
    emit_config: Arc<Config>,
) -> DiagnosticsHandler {
    create_diagnostics_handler(
        options,
        Default::default(),
        output_writer,
        error_writer,
        emit_config,
    )
}

pub(super) fn create_diagnostics_handler(
    options: &Options,
    codemap: Arc<CodeMap>,
    output_writer: Arc<Mutex<dyn WriteColor>>,
    error_writer: Arc<Mutex<dyn WriteColor>>,
    emit_config: Arc<Config>,
) -> DiagnosticsHandler {
    let config = DiagnosticsConfig {
        warnings_as_errors: options.warnings_as_errors,
        no_warn: options.no_warn,
    };
    DiagnosticsHandler::new(config, codemap, output_writer, error_writer, emit_config)
}

pub(super) fn abort_on_err<T>(_: ()) -> T {
    use liblumen_util::error::FatalError;

    FatalError.raise()
}

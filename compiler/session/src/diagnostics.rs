use std::error::Error;
use std::ffi::CString;
use std::fmt::Display;
use std::io;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use codespan_reporting::term;
use codespan_reporting::term::Config;

use libeir_diagnostics::{CodeMap, Diagnostic, Severity};

use liblumen_util::error::{FatalError, Verbosity};

use termcolor::{Color, ColorSpec, WriteColor};

#[derive(Debug, Copy, Clone)]
pub struct DiagnosticsConfig {
    pub warnings_as_errors: bool,
    pub no_warn: bool,
}

#[repr(C)]
pub struct Location {
    pub file: CString,
    pub line: u32,
    pub column: u32,
}
impl Location {
    pub fn new(file: CString, line: u32, column: u32) -> Self {
        Self { file, line, column }
    }
}

#[derive(Clone)]
pub struct DiagnosticsHandler {
    output_writer: Arc<Mutex<dyn WriteColor>>,
    error_writer: Arc<Mutex<dyn WriteColor>>,
    emit_config: Arc<Config>,
    codemap: Arc<CodeMap>,
    warnings_as_errors: bool,
    no_warn: bool,
    err_count: Arc<AtomicUsize>,
}
// We can safely implement these traits for DiagnosticsHandler,
// as the only two non-atomic fields are read-only after creation
unsafe impl Send for DiagnosticsHandler {}
unsafe impl Sync for DiagnosticsHandler {}
impl DiagnosticsHandler {
    pub fn new(
        config: DiagnosticsConfig,
        codemap: Arc<CodeMap>,
        output_writer: Arc<Mutex<dyn WriteColor>>,
        error_writer: Arc<Mutex<dyn WriteColor>>,
        emit_config: Arc<Config>,
    ) -> Self {
        Self {
            output_writer,
            error_writer,
            emit_config,
            codemap,
            warnings_as_errors: config.warnings_as_errors,
            no_warn: config.no_warn,
            err_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn has_errors(&self) -> bool {
        self.err_count.load(Ordering::Relaxed) > 0
    }

    pub fn abort_if_errors(&self) {
        if self.has_errors() {
            FatalError.raise();
        }
    }

    pub fn fatal<E>(&self, err: E) -> FatalError
    where
        E: Deref<Target = (dyn Error + Send + Sync + 'static)>,
    {
        self.write_error(err).unwrap();
        FatalError
    }

    pub fn fatal_str(&self, err: &str) -> FatalError {
        self.diagnostic(&Diagnostic::error().with_message(err))
            .unwrap();

        FatalError
    }

    pub fn error<E>(&self, err: E) -> io::Result<()>
    where
        E: Deref<Target = (dyn Error + Send + Sync + 'static)>,
    {
        self.err_count.fetch_add(1, Ordering::Relaxed);
        self.write_error(err)
    }

    pub fn io_error(&self, err: std::io::Error) -> io::Result<()> {
        self.err_count.fetch_add(1, Ordering::Relaxed);
        let e: &(dyn std::error::Error + Send + Sync + 'static) = &err;
        self.write_error(e)
    }

    pub fn error_str(&self, err: &str) -> io::Result<()> {
        self.err_count.fetch_add(1, Ordering::Relaxed);
        self.diagnostic(&Diagnostic::error().with_message(err))
    }

    pub fn warn<M: Display>(&self, message: M) -> io::Result<()> {
        let option_diagnostic = if self.warnings_as_errors {
            Some(Diagnostic::error())
        } else if !self.no_warn {
            Some(Diagnostic::warning())
        } else {
            None
        };

        match option_diagnostic {
            None => Ok(()),
            Some(diagnostic) => self.diagnostic(&diagnostic.with_message(message.to_string())),
        }
    }

    pub fn diagnostic(&self, diagnostic: &Diagnostic) -> io::Result<()> {
        let mut writer = match diagnostic.severity {
            Severity::Bug | Severity::Error | Severity::Warning => {
                self.error_writer.lock().unwrap()
            }
            Severity::Note | Severity::Help => self.output_writer.lock().unwrap(),
        };

        term::emit(&mut *writer, &self.emit_config, &*self.codemap, diagnostic)
    }

    pub fn success<M: Display>(&self, prefix: &str, message: M) -> io::Result<()> {
        Self::write_prefixed(&self.output_writer, green_bold(), prefix, message)
    }

    pub fn failed<M: Display>(&self, prefix: &str, message: M) -> io::Result<()> {
        self.err_count.fetch_add(1, Ordering::Relaxed);
        Self::write_prefixed(&self.error_writer, red_bold(), prefix, message)
    }

    pub fn info<M: Display>(&self, message: M) -> io::Result<()> {
        self.write_info(cyan(), message)
    }

    pub fn debug<M: Display>(&self, message: M) -> io::Result<()> {
        self.write_debug(white(), message)
    }

    fn write_error<E>(&self, err: E) -> io::Result<()>
    where
        E: Deref<Target = (dyn Error + Send + Sync + 'static)>,
    {
        self.diagnostic(&Diagnostic::error().with_message(err.deref().to_string()))
    }

    fn write_prefixed<M: Display>(
        mutex_writer: &Mutex<dyn WriteColor>,
        color: ColorSpec,
        prefix: &str,
        message: M,
    ) -> io::Result<()> {
        let mut writer = mutex_writer.lock().unwrap();
        writer.set_color(&color)?;
        write!(writer, "{:>12} ", prefix)?;
        writer.reset()?;
        write!(writer, "{}\n", message)
    }

    fn write_info<M: Display>(&self, color: ColorSpec, message: M) -> io::Result<()> {
        let mut writer = self.output_writer.lock().unwrap();
        writer.set_color(&color)?;
        write!(writer, "{}", message)?;
        writer.reset()?;

        Ok(())
    }

    fn write_debug<M: Display>(&self, color: ColorSpec, message: M) -> io::Result<()> {
        let mut writer = self.error_writer.lock().unwrap();
        writer.set_color(&color)?;
        write!(writer, "{}", message)?;
        writer.reset()?;

        Ok(())
    }
}

fn color(color: Color) -> ColorSpec {
    let mut color_spec = ColorSpec::new();
    color_spec.set_fg(Some(color));

    color_spec
}

fn color_bold(color: Color) -> ColorSpec {
    let mut color_spec = ColorSpec::new();
    color_spec
        .set_fg(Some(color))
        .set_bold(true)
        .set_intense(true);

    color_spec
}

fn cyan() -> ColorSpec {
    if cfg!(windows) {
        color(Color::Cyan)
    } else {
        color(Color::Blue)
    }
}

fn green_bold() -> ColorSpec {
    color_bold(Color::Green)
}

fn red_bold() -> ColorSpec {
    color_bold(Color::Red)
}

fn white() -> ColorSpec {
    color(Color::White)
}

pub fn verbosity_to_severity(v: Verbosity) -> Severity {
    match v {
        Verbosity::Silent => Severity::Bug,
        Verbosity::Error => Severity::Error,
        Verbosity::Warning => Severity::Warning,
        Verbosity::Info => Severity::Note,
        Verbosity::Debug => Severity::Note,
    }
}

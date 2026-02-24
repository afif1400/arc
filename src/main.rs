use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "arc")]
#[command(version)]
#[command(about = "A minimal, LLM-native architecture diagram language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render an .arc file to SVG or PNG
    Render {
        /// Input .arc file (reads from stdin if omitted)
        #[arg(value_name = "FILE")]
        input: Option<PathBuf>,

        /// Output file path (writes to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long, default_value = "svg")]
        format: String,

        /// Theme: light, dark, blueprint, mono
        #[arg(short, long)]
        theme: Option<String>,
    },

    /// Validate an .arc file and report errors
    Validate {
        /// Input .arc file (reads from stdin if omitted)
        #[arg(value_name = "FILE")]
        input: Option<PathBuf>,

        /// Output format: human, json
        #[arg(short, long, default_value = "human")]
        format: String,

        /// Auto-fix common errors
        #[arg(long)]
        fix: bool,
    },

    /// Format an .arc file with consistent style
    Fmt {
        /// Input .arc file
        #[arg(value_name = "FILE")]
        input: Option<PathBuf>,

        /// Write result back to file (in-place)
        #[arg(short, long)]
        write: bool,
    },

    /// Start the MCP server for AI agent integration
    Mcp,

    /// Print the arc grammar specification (for LLM system prompts)
    Grammar,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Render { input, output, format, theme } => {
            let source = read_input(input.as_deref());
            let theme_name = theme.as_deref();

            let result = arc_lang::render(&source, theme_name);

            // Print diagnostics to stderr
            for diag in &result.diagnostics {
                print_diagnostic(diag);
            }

            if format == "svg" {
                write_output(output.as_deref(), &result.svg);
            } else {
                eprintln!("Error: only 'svg' format is supported in v0.1 (PNG coming soon)");
                process::exit(1);
            }
        }

        Commands::Validate { input, format, fix: _ } => {
            let source = read_input(input.as_deref());
            let parse_result = arc_lang::parser::parse(&source);
            let (validation, _resolved) = arc_lang::validator::validate(
                &parse_result.document,
                &parse_result.diagnostics,
            );

            if format == "json" {
                let json = serde_json::to_string_pretty(&validation).unwrap();
                println!("{}", json);
            } else {
                // Human-readable output
                if validation.errors.is_empty() {
                    println!("\x1b[32m✓\x1b[0m No issues found.");
                } else {
                    for diag in &validation.errors {
                        print_diagnostic(diag);
                    }
                    let errors = validation.errors.iter()
                        .filter(|d| d.severity == arc_lang::ast::Severity::Error).count();
                    let warnings = validation.errors.iter()
                        .filter(|d| d.severity == arc_lang::ast::Severity::Warning).count();
                    let infos = validation.errors.iter()
                        .filter(|d| d.severity == arc_lang::ast::Severity::Info).count();

                    println!();
                    if errors > 0 { println!("\x1b[31m{} error(s)\x1b[0m", errors); }
                    if warnings > 0 { println!("\x1b[33m{} warning(s)\x1b[0m", warnings); }
                    if infos > 0 { println!("\x1b[36m{} info(s)\x1b[0m", infos); }
                }

                if !validation.valid {
                    process::exit(1);
                }
            }
        }

        Commands::Fmt { input, write } => {
            let source = read_input(input.as_deref());
            let parse_result = arc_lang::parser::parse(&source);
            let formatted = arc_lang::fmt::format_document(&parse_result.document);

            if write {
                if let Some(path) = input {
                    fs::write(&path, &formatted).unwrap_or_else(|e| {
                        eprintln!("Error writing {}: {}", path.display(), e);
                        process::exit(1);
                    });
                    println!("Formatted {}", path.display());
                } else {
                    eprintln!("Error: --write requires a file path");
                    process::exit(1);
                }
            } else {
                print!("{}", formatted);
            }
        }

        Commands::Mcp => {
            arc_lang::mcp::run_mcp_server().unwrap_or_else(|e| {
                eprintln!("MCP server error: {}", e);
                process::exit(1);
            });
        }

        Commands::Grammar => {
            print!("{}", arc_lang::mcp::GRAMMAR_SPEC);
        }
    }
}

fn read_input(path: Option<&std::path::Path>) -> String {
    match path {
        Some(p) => {
            fs::read_to_string(p).unwrap_or_else(|e| {
                eprintln!("Error reading {}: {}", p.display(), e);
                process::exit(1);
            })
        }
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                eprintln!("Error reading stdin: {}", e);
                process::exit(1);
            });
            buf
        }
    }
}

fn write_output(path: Option<&std::path::Path>, content: &str) {
    match path {
        Some(p) => {
            fs::write(p, content).unwrap_or_else(|e| {
                eprintln!("Error writing {}: {}", p.display(), e);
                process::exit(1);
            });
            eprintln!("Wrote {}", p.display());
        }
        None => {
            print!("{}", content);
        }
    }
}

fn print_diagnostic(diag: &arc_lang::ast::Diagnostic) {
    let (prefix, color) = match diag.severity {
        arc_lang::ast::Severity::Error   => ("error", "\x1b[31m"),
        arc_lang::ast::Severity::Warning => ("warn ", "\x1b[33m"),
        arc_lang::ast::Severity::Info    => ("info ", "\x1b[36m"),
    };

    eprintln!("{}{}[{}]\x1b[0m line {}:{} — {}",
        color, prefix, diag.code, diag.line, diag.col, diag.message);

    if let Some(ref suggestion) = diag.suggestion {
        eprintln!("  \x1b[2m→ suggestion: {}\x1b[0m", suggestion);
    }
}

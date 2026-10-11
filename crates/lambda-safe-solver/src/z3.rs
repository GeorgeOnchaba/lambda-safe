use crate::query::{DecisionProcedure, DecisionResult, SolverError, SolverQuery};
use crate::smtlib::emit_smt2;
use lambda_safe_model::{Counterexample, PrimitiveValue, SolverIdentity};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Z3 adapter using the stable SMT-LIB process boundary.
#[derive(Clone, Debug)]
pub struct Z3ProcessSolver {
    executable: PathBuf,
    version: String,
}

impl Z3ProcessSolver {
    /// Discovers and records the Z3 version. Construction fails when the binary
    /// is unavailable so readiness can fail before accepting compile traffic.
    pub fn discover(path: impl Into<PathBuf>) -> Result<Self, SolverError> {
        let executable = path.into();
        let output = Command::new(&executable)
            .arg("-version")
            .output()
            .map_err(|error| SolverError::Io(format!("cannot execute Z3: {error}")))?;
        if !output.status.success() {
            return Err(SolverError::Io(format!(
                "Z3 version command exited with {}",
                output.status
            )));
        }
        let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if version.is_empty() {
            return Err(SolverError::Protocol(
                "Z3 returned an empty version string".to_owned(),
            ));
        }
        Ok(Self {
            executable,
            version,
        })
    }

    /// Creates an adapter with an externally established version. This is
    /// useful for dependency injection and does not bypass solve-time failures.
    #[must_use]
    pub fn with_version(path: impl Into<PathBuf>, version: impl Into<String>) -> Self {
        Self {
            executable: path.into(),
            version: version.into(),
        }
    }

    /// Configured binary path.
    #[must_use]
    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

impl DecisionProcedure for Z3ProcessSolver {
    fn identity(&self) -> SolverIdentity {
        SolverIdentity {
            name: "z3-smtlib-process".to_owned(),
            version: self.version.clone(),
            theory: "QF_LIA".to_owned(),
            options: BTreeMap::from([
                ("auto_config".to_owned(), "false".to_owned()),
                ("produce_models".to_owned(), "true".to_owned()),
            ]),
        }
    }

    fn solve(&self, query: &SolverQuery, timeout: Duration) -> Result<DecisionResult, SolverError> {
        let started = Instant::now();
        let script = emit_smt2(query, timeout);
        let output = self.run_process(&script, timeout)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure_success(output.status, &stderr)?;

        let first = stdout.lines().next().unwrap_or_default().trim();
        match first {
            "unsat" => Ok(DecisionResult::Unsat),
            "unknown" => Ok(DecisionResult::Unknown(if stderr.trim().is_empty() {
                "Z3 returned unknown".to_owned()
            } else {
                truncate(stderr.trim(), 2_048)
            })),
            "sat" => {
                let remaining = timeout
                    .checked_sub(started.elapsed())
                    .ok_or(SolverError::Timeout(timeout.as_millis() as u64))?;
                if remaining.is_zero() {
                    return Err(SolverError::Timeout(timeout.as_millis() as u64));
                }
                let mut model_script = emit_smt2(query, remaining);
                model_script.push_str("(get-model)\n");
                let model_output = self.run_process(&model_script, remaining)?;
                let model_stdout = String::from_utf8_lossy(&model_output.stdout);
                let model_stderr = String::from_utf8_lossy(&model_output.stderr);
                ensure_success(model_output.status, &model_stderr)?;
                if model_stdout.lines().next().unwrap_or_default().trim() != "sat" {
                    return Err(SolverError::Protocol(
                        "Z3 changed result while retrieving the model".to_owned(),
                    ));
                }
                Ok(DecisionResult::Sat(Counterexample {
                    assignments: parse_simple_model(&model_stdout),
                    raw_model: Some(truncate(&model_stdout, 32_768)),
                }))
            }
            other => Err(SolverError::Protocol(format!(
                "expected sat, unsat or unknown; received `{}`",
                truncate(other, 256)
            ))),
        }
    }
}

impl Z3ProcessSolver {
    fn run_process(&self, script: &str, timeout: Duration) -> Result<ProcessOutput, SolverError> {
        let mut child = Command::new(&self.executable)
            .args(["-in", "-smt2"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| SolverError::Io(format!("cannot start Z3: {error}")))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| SolverError::Io("Z3 stdin was not captured".to_owned()))?;
        let input = script.as_bytes().to_vec();
        let stdin_writer = thread::spawn(move || stdin.write_all(&input));

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| SolverError::Io("Z3 stdout was not captured".to_owned()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| SolverError::Io("Z3 stderr was not captured".to_owned()))?;
        let stdout_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });

        let started = Instant::now();
        let status = loop {
            match child
                .try_wait()
                .map_err(|error| SolverError::Io(format!("cannot poll Z3: {error}")))?
            {
                Some(status) => break status,
                None if started.elapsed() >= timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdin_writer.join();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(SolverError::Timeout(timeout.as_millis() as u64));
                }
                None => thread::sleep(Duration::from_millis(2)),
            }
        };

        stdin_writer
            .join()
            .map_err(|_| SolverError::Io("Z3 stdin writer panicked".to_owned()))?
            .map_err(|error| SolverError::Io(format!("cannot write SMT-LIB input: {error}")))?;
        let stdout = stdout_reader
            .join()
            .map_err(|_| SolverError::Io("Z3 stdout reader panicked".to_owned()))?
            .map_err(|error| SolverError::Io(format!("cannot read Z3 stdout: {error}")))?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| SolverError::Io("Z3 stderr reader panicked".to_owned()))?
            .map_err(|error| SolverError::Io(format!("cannot read Z3 stderr: {error}")))?;
        Ok(ProcessOutput {
            status,
            stdout,
            stderr,
        })
    }
}

#[derive(Debug)]
struct ProcessOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn ensure_success(status: std::process::ExitStatus, stderr: &str) -> Result<(), SolverError> {
    if status.success() {
        Ok(())
    } else {
        Err(SolverError::Protocol(format!(
            "Z3 exited with {status}: {}",
            truncate(stderr.trim(), 2_048)
        )))
    }
}

fn parse_simple_model(output: &str) -> BTreeMap<String, PrimitiveValue> {
    let spaced = output.replace('(', " ( ").replace(')', " ) ");
    let tokens = spaced.split_whitespace().collect::<Vec<_>>();
    let mut values = BTreeMap::new();
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index] != "define-fun" || index + 5 >= tokens.len() {
            index += 1;
            continue;
        }
        let name = decode_model_name(tokens[index + 1]);
        let mut sort_index = index + 2;
        while sort_index < tokens.len()
            && tokens[sort_index] != "Int"
            && tokens[sort_index] != "Bool"
        {
            sort_index += 1;
        }
        if sort_index + 1 >= tokens.len() {
            break;
        }
        match tokens[sort_index] {
            "Bool" => {
                if let Ok(value) = tokens[sort_index + 1].parse::<bool>() {
                    let _ = values.insert(name.clone(), PrimitiveValue::Boolean(value));
                }
            }
            "Int" => {
                let first = tokens[sort_index + 1];
                let parsed = if first == "(" && sort_index + 3 < tokens.len() {
                    if tokens[sort_index + 2] == "-" {
                        tokens[sort_index + 3]
                            .parse::<i64>()
                            .ok()
                            .and_then(i64::checked_neg)
                    } else {
                        None
                    }
                } else {
                    first.parse::<i64>().ok()
                };
                if let Some(value) = parsed {
                    let _ = values.insert(name.clone(), PrimitiveValue::Integer(value));
                }
            }
            _ => {}
        }
        index = sort_index + 1;
    }
    values
}

fn decode_model_name(value: &str) -> String {
    let value = value.trim_matches('|');
    let Some(encoded) = value.strip_prefix("ls_") else {
        return value.to_owned();
    };
    let (hex, next) = if let Some(hex) = encoded.strip_suffix("_pre") {
        (hex, false)
    } else if let Some(hex) = encoded.strip_suffix("_next") {
        (hex, true)
    } else {
        return value.to_owned();
    };
    if hex.len() % 2 != 0 {
        return value.to_owned();
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>();
    let Ok(bytes) = bytes else {
        return value.to_owned();
    };
    let Ok(mut name) = String::from_utf8(bytes) else {
        return value.to_owned();
    };
    if next {
        name.push_str("__next");
    }
    name
}

fn truncate(value: &str, maximum: usize) -> String {
    if value.len() <= maximum {
        value.to_owned()
    } else {
        let mut end = maximum;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &value[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_integer_and_boolean_model() {
        let model =
            "sat\n(\n (define-fun x () Int\n  (- 2))\n (define-fun enabled () Bool\n  true)\n)";
        let parsed = parse_simple_model(model);
        assert_eq!(parsed.get("x"), Some(&PrimitiveValue::Integer(-2)));
        assert_eq!(parsed.get("enabled"), Some(&PrimitiveValue::Boolean(true)));
    }

    #[test]
    fn decodes_solver_symbol_names() {
        assert_eq!(decode_model_name("|ls_78_pre|"), "x");
        assert_eq!(decode_model_name("|ls_78_next|"), "x__next");
    }

    #[test]
    fn truncation_preserves_utf8_boundaries() {
        let value = "λλλλ";
        let truncated = truncate(value, 3);
        assert!(truncated.is_char_boundary(truncated.len()));
    }
}

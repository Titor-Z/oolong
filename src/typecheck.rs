use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub message: String,
}

/// Run TypeScript type checking using the user-installed `tsgo` binary.
pub fn type_check(file_path: &str) -> Result<Vec<Diagnostic>, String> {
    let tsgo_path = find_tsgo()?;

    let output = Command::new(&tsgo_path)
        .args(["--noEmit", file_path])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("failed to run tsgo: {e}"))?;

    if output.status.success() {
        return Ok(Vec::new());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(parse_diagnostics(&stderr))
}

fn find_tsgo() -> Result<String, String> {
    if let Some(home) = home_dir() {
        let local_path = Path::new(&home).join(".oolong").join("tsgo");
        if local_path.exists() {
            return Ok(local_path.to_str().unwrap().to_string());
        }
    }

    if let Some(path) = find_in_path("tsgo") {
        return Ok(path);
    }

    Err(
    "tsgo not found. Please install it:\n  mkdir -p ~/.oolong\n  cp /path/to/tsgo ~/.oolong/tsgo"
      .to_string(),
  )
}

fn home_dir() -> Option<String> {
    std::env::var("HOME").ok()
}

fn find_in_path(name: &str) -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    for dir in path.split(':') {
        let candidate = Path::new(dir).join(name);
        if candidate.exists() {
            return candidate.to_str().map(|s| s.to_string());
        }
    }
    None
}

fn parse_diagnostics(stderr: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for line in stderr.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_suffix(')')
            && let Some(pos) = rest.rfind('(')
        {
            let coords = &rest[pos + 1..];
            if let Some((line_str, col_str)) = coords.rsplit_once(',')
                && let (Ok(line_num), Ok(col_num)) = (
                    line_str.trim().parse::<u32>(),
                    col_str.trim().parse::<u32>(),
                )
            {
                let file = rest[..pos].trim().to_string();
                let msg_start = line.find("error ").unwrap_or(0);
                diags.push(Diagnostic {
                    file,
                    line: line_num,
                    col: col_num,
                    message: line[msg_start..].to_string(),
                });
                continue;
            }
        }

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() == 4
            && let (Ok(line_num), Ok(col_num)) = (
                parts[1].trim().parse::<u32>(),
                parts[2].trim().parse::<u32>(),
            )
        {
            let diag_msg = if let Some(pos) = parts[3].find("error ") {
                parts[3][pos..].to_string()
            } else {
                parts[3].to_string()
            };
            diags.push(Diagnostic {
                file: parts[0].trim().to_string(),
                line: line_num,
                col: col_num,
                message: diag_msg,
            });
        }
    }
    diags
}

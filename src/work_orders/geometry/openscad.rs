//! OpenSCAD execution and volume evaluation engine.
//!
//! Submits variables into OpenSCAD, renders to STL with a strict process timeout,
//! and parses the resulting mesh using `stl_io` to compute total 3D volume.
//! If OpenSCAD is unavailable or fails, returns an error directly.

use std::collections::HashMap;
use std::time::Duration;
use tokio::fs;

use super::stl::{StlError, calculate_stl_volume};

#[derive(Debug, thiserror::Error)]
pub enum OpenScadError {
    #[error("OpenSCAD execution timed out after {0:?}")]
    Timeout(Duration),
    #[error("OpenSCAD binary not found on server. Please install 'openscad': {0}")]
    NotFound(String),
    #[error("OpenSCAD failed with exit code {0}: {1}")]
    ProcessFailed(i32, String),
    #[error("I/O error during OpenSCAD execution: {0}")]
    Io(#[from] std::io::Error),
    #[error("STL calculation error: {0}")]
    Stl(#[from] StlError),
    #[error("Invalid variable name '{0}': only alphanumeric characters and underscores are permitted")]
    InvalidVariableName(String),
}

/// Substitute variables in OpenSCAD code and calculate the 3D volume in meters³.
///
/// Runs `openscad -o out.stl -D "var=val" ...` asynchronously.
/// Returns an error if OpenSCAD is unavailable or fails.
pub async fn calculate_openscad_volume(
    code: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, OpenScadError> {
    // 1. Validate variable names against script injection
    for (name, val) in variables {
        if !is_safe_var_name(name) {
            return Err(OpenScadError::InvalidVariableName(name.clone()));
        }
        if val.is_nan() || val.is_infinite() {
            return Err(OpenScadError::InvalidVariableName(format!(
                "{} has invalid float value",
                name
            )));
        }
    }

    execute_openscad_cli(code, variables).await
}

/// Asynchronously invoke the `openscad` CLI with timeout and temporary files.
async fn execute_openscad_cli(
    code: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, OpenScadError> {
    let tmp_dir = std::env::temp_dir();
    let unique_id = uuid::Uuid::new_v4();
    let scad_path = tmp_dir.join(format!("lariv_scad_{}.scad", unique_id));
    let stl_path = tmp_dir.join(format!("lariv_stl_{}.stl", unique_id));

    // Write input SCAD file
    fs::write(&scad_path, code).await?;

    // Build command with -D parameter overrides
    let mut cmd = tokio::process::Command::new("openscad");
    cmd.arg("-o").arg(&stl_path);

    for (name, val) in variables {
        cmd.arg("-D").arg(format!("{}={}", name, val));
    }
    cmd.arg(&scad_path);

    // Run with 5-second timeout
    let timeout_duration = Duration::from_secs(5);
    let run_res = tokio::time::timeout(timeout_duration, cmd.output()).await;

    // Clean up scad file regardless of outcome
    let _ = fs::remove_file(&scad_path).await;

    let output = match run_res {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            let _ = fs::remove_file(&stl_path).await;
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(OpenScadError::NotFound(
                    "The 'openscad' executable was not found. Please install openscad.".into(),
                ));
            }
            return Err(OpenScadError::Io(e));
        }
        Err(_) => {
            let _ = fs::remove_file(&stl_path).await;
            return Err(OpenScadError::Timeout(timeout_duration));
        }
    };

    if !output.status.success() {
        let _ = fs::remove_file(&stl_path).await;
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(OpenScadError::ProcessFailed(
            output.status.code().unwrap_or(-1),
            stderr,
        ));
    }

    // Read generated STL and compute volume in mm³, then convert to m³
    let stl_bytes = fs::read(&stl_path).await?;
    let _ = fs::remove_file(&stl_path).await;

    let volume_mm3 = calculate_stl_volume(&stl_bytes)?;
    Ok(volume_mm3 * 1e-9)
}

fn is_safe_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

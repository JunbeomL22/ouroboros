use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn call_claude(role: &str, prompt: &str, model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    // Use stdin to pass the prompt - avoids escaping issues on all platforms
    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args(["/C", "claude", "-p", "-", "--model", model, "--dangerously-skip-permissions"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn claude CLI")?;

    #[cfg(not(windows))]
    let mut child = Command::new("claude")
        .args(["-p", "-", "--model", model, "--dangerously-skip-permissions"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn claude CLI")?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(full_prompt.as_bytes())
            .context("Failed to write prompt to claude stdin")?;
    }

    let output = child.wait_with_output().context("Failed to wait for claude CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude CLI failed: {}", stderr);
    }

    let response = String::from_utf8(output.stdout)
        .context("Failed to parse claude output as UTF-8")?;

    Ok(response.trim().to_string())
}

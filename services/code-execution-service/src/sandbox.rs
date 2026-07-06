use bollard::{
    container::{
        Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
        StartContainerOptions, WaitContainerOptions,
    },
    models::HostConfig,
    Docker,
    image::CreateImageOptions,
};
use futures_util::StreamExt;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

const MAX_MEMORY_BYTES: i64 = 128 * 1024 * 1024; // 128 MB
const CPU_QUOTA: i64 = 50_000; // 50% of one CPU core
const CPU_PERIOD: i64 = 100_000;

pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
    pub execution_time_ms: u128,
}

pub enum SandboxError {
    UnsupportedLanguage(String),
    Timeout(u64),
    DockerError(String),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxError::UnsupportedLanguage(lang) => write!(f, "Unsupported language: {lang}"),
            SandboxError::Timeout(secs) => write!(f, "Execution timed out after {secs}s"),
            SandboxError::DockerError(msg) => write!(f, "Docker error: {msg}"),
        }
    }
}

fn resolve_image_and_cmd(language: &str, code: &str) -> Option<(&'static str, Vec<String>)> {
    match language {
        "python" => Some((
            "python:3.12-slim",
            vec!["python3".into(), "-c".into(), code.into()],
        )),
        "javascript" => Some((
            "node:20-slim",
            vec!["node".into(), "-e".into(), code.into()],
        )),
        _ => None,
    }
}

pub async fn run_in_sandbox(
    docker: &Docker,
    language: &str,
    code: &str,
    timeout_seconds: u64,
) -> Result<ExecutionResult, SandboxError> {
    let (image, cmd) = resolve_image_and_cmd(language, code)
        .ok_or_else(|| SandboxError::UnsupportedLanguage(language.to_string()))?;

    let mut pull_stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: image,
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(pull_result) = pull_stream.next().await {
        if let Err(e) = pull_result {
            return Err(SandboxError::DockerError(format!("create_image (pull): {e}")));
        }
    }

    let container_name = format!("sandbox-{}", Uuid::new_v4());

    let container_id = docker
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.as_str(),
                platform: None,
            }),
            Config {
                image: Some(image),
                cmd: Some(cmd.iter().map(String::as_str).collect()),
                host_config: Some(HostConfig {
                    memory: Some(MAX_MEMORY_BYTES),
                    cpu_quota: Some(CPU_QUOTA),
                    cpu_period: Some(CPU_PERIOD),
                    network_mode: Some("none".into()),
                    auto_remove: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| SandboxError::DockerError(format!("create_container: {e}")))?
        .id;

    docker
        .start_container(&container_id, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| SandboxError::DockerError(format!("start_container: {e}")))?;

    let start = Instant::now();

    // Wait for the container to exit, enforcing a hard time limit.
    let timed_out = timeout(Duration::from_secs(timeout_seconds), async {
        let mut stream = docker.wait_container(
            &container_id,
            None::<WaitContainerOptions<String>>,
        );
        stream.next().await
    })
    .await
    .is_err();

    let execution_time_ms = start.elapsed().as_millis();

    if timed_out {
        let _ = docker
            .remove_container(
                &container_id,
                Some(RemoveContainerOptions { force: true, ..Default::default() }),
            )
            .await;
        return Err(SandboxError::Timeout(timeout_seconds));
    }

    // Collect stdout and stderr from container logs.
    let mut stdout = String::new();
    let mut stderr = String::new();

    let mut log_stream = docker.logs(
        &container_id,
        Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        }),
    );

    while let Some(chunk) = log_stream.next().await {
        match chunk {
            Ok(LogOutput::StdOut { message }) => {
                stdout.push_str(&String::from_utf8_lossy(&message));
            }
            Ok(LogOutput::StdErr { message }) => {
                stderr.push_str(&String::from_utf8_lossy(&message));
            }
            _ => {}
        }
    }

    let inspect = docker
        .inspect_container(&container_id, None)
        .await
        .map_err(|e| SandboxError::DockerError(format!("inspect_container: {e}")))?;

    let exit_code = inspect
        .state
        .and_then(|s| s.exit_code)
        .unwrap_or(-1);

    let _ = docker
        .remove_container(
            &container_id,
            Some(RemoveContainerOptions { force: true, ..Default::default() }),
        )
        .await;

    Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        execution_time_ms,
    })
}

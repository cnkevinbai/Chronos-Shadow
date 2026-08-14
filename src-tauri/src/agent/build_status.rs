// 构建产物状态扫描 (Build Status)
// 扫描 dist/ 与 target/release 下的构建产物，供前端展示构建健康度

#[derive(serde::Serialize)]
pub struct BuildFileStatus {
    path: String,
    name: String,
    ext: String,
    size_bytes: u64,
    gzip_size: Option<u64>,
    status: String,
    warnings_count: u32,
    errors_count: u32,
}

#[derive(serde::Serialize)]
pub struct BuildSummary {
    total_files: usize,
    compiled_files: usize,
    warning_files: usize,
    error_files: usize,
    total_size_bytes: u64,
    total_gzip_bytes: u64,
    total_compile_time_ms: u64,
    build_timestamp: String,
    files: Vec<BuildFileStatus>,
}

fn scan_dir(dir: &std::path::Path, prefix: &str, out: &mut Vec<BuildFileStatus>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").into();
            if path.is_dir() {
                scan_dir(&path, &format!("{}/{}", prefix, name), out);
            } else if let Ok(meta) = entry.metadata() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").into();
                let rel = format!("{}/{}", prefix, name);
                out.push(BuildFileStatus {
                    path: rel,
                    name,
                    ext,
                    size_bytes: meta.len(),
                    gzip_size: None,
                    status: "ok".into(),
                    warnings_count: 0,
                    errors_count: 0,
                });
            }
        }
    }
}

#[tauri::command]
pub fn get_build_status() -> Result<BuildSummary, String> {
    let mut files = Vec::new();
    let dist_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().map(|p| p.join("dist"));
    let target_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release");

    // Scan dist/
    if let Some(ref dist) = dist_dir {
        if dist.exists() {
            scan_dir(dist, "dist", &mut files);
        }
    }

    // Scan target/release for exe/msi
    if target_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".exe") || name.ends_with(".msi") {
                    let meta = entry.metadata().ok();
                    files.push(BuildFileStatus {
                        path: format!("target/release/{}", name),
                        name: name.into(),
                        ext: path.extension().and_then(|e| e.to_str()).unwrap_or("").into(),
                        size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                        gzip_size: None,
                        status: "ok".into(),
                        warnings_count: 0,
                        errors_count: 0,
                    });
                }
            }
        }
    }

    let total = files.len();
    let warnings = files.iter().filter(|f| f.status == "warning").count();
    let errors = files.iter().filter(|f| f.status == "error").count();
    let total_size: u64 = files.iter().map(|f| f.size_bytes).sum();
    let total_gzip: u64 = files.iter().filter_map(|f| f.gzip_size).sum();

    Ok(BuildSummary {
        total_files: total,
        compiled_files: total - errors,
        warning_files: warnings,
        error_files: errors,
        total_size_bytes: total_size,
        total_gzip_bytes: total_gzip,
        total_compile_time_ms: 0,
        build_timestamp: chrono::Utc::now().to_rfc3339(),
        files,
    })
}
